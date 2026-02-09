use reqwest::{Client, Method, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{Api, Environment, GherkinStep, Scenario, TestExample};

/// Configuration for test execution
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub timeout: Duration,
    pub auth_token: Option<String>,
    pub custom_headers: HashMap<String, String>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            auth_token: None,
            custom_headers: HashMap::new(),
        }
    }
}

/// Result of a single test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub scenario_id: Uuid,
    pub api_id: Uuid,
    pub example_index: i32,
    pub pass: bool,
    pub error_message: Option<String>,
    pub response_status: i16,
    pub response_data: Option<serde_json::Value>,
    pub request_duration_ms: i64,
    pub request_time: time::OffsetDateTime,
}

/// Context built from Gherkin steps
#[derive(Debug, Clone, Default)]
pub struct StepContext {
    pub request_body: Option<serde_json::Value>,
    pub request_headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub path_params: HashMap<String, String>,
    pub expected_status: Option<i16>,
    pub expected_body: Option<serde_json::Value>,
    pub expected_body_contains: Vec<String>,
    /// Data table from step (for setup data)
    pub setup_data: Option<Vec<serde_json::Value>>,
}

/// Test Runner Service
pub struct TestRunner {
    client: Client,
    config: TestConfig,
}

impl TestRunner {
    /// Create a new TestRunner with default config
    pub fn new() -> Self {
        Self::with_config(TestConfig::default())
    }

    /// Create a new TestRunner with custom config
    pub fn with_config(config: TestConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Run all examples for a scenario
    pub async fn run_scenario(
        &self,
        scenario: &Scenario,
        api: &Api,
        environment: &Environment,
    ) -> AppResult<Vec<TestResult>> {
        let steps: Vec<GherkinStep> = serde_json::from_value(scenario.steps.clone())
            .map_err(|e| AppError::Validation(format!("Invalid steps format: {}", e)))?;

        let examples: Vec<TestExample> = serde_json::from_value(scenario.examples.clone())
            .map_err(|e| AppError::Validation(format!("Invalid examples format: {}", e)))?;

        let mut results = Vec::new();

        for (index, example) in examples.iter().enumerate() {
            let result = self
                .run_example(scenario, api, environment, &steps, example, index as i32)
                .await;
            results.push(result);
        }

        Ok(results)
    }

    /// Run a single example
    async fn run_example(
        &self,
        scenario: &Scenario,
        api: &Api,
        environment: &Environment,
        steps: &[GherkinStep],
        example: &TestExample,
        example_index: i32,
    ) -> TestResult {
        let request_time = time::OffsetDateTime::now_utc();
        let start = Instant::now();

        // Build context from steps
        let mut context = StepContext::default();
        context.expected_status = Some(example.expected_status_code);
        context.expected_body = Some(example.expected_response_body.clone());

        // Process steps to build context
        for step in steps {
            self.process_step(&mut context, step, &example.example);
        }

        // Build and execute request
        let result = self
            .execute_request(api, environment, &context, &example.example)
            .await;

        let duration = start.elapsed().as_millis() as i64;

        match result {
            Ok((status, body)) => {
                // Validate response
                let validation = self.validate_response(status, &body, &context);

                TestResult {
                    scenario_id: scenario.id,
                    api_id: api.id,
                    example_index,
                    pass: validation.is_ok(),
                    error_message: validation.err(),
                    response_status: status,
                    response_data: Some(body),
                    request_duration_ms: duration,
                    request_time,
                }
            }
            Err(e) => TestResult {
                scenario_id: scenario.id,
                api_id: api.id,
                example_index,
                pass: false,
                error_message: Some(e.to_string()),
                response_status: 0,
                response_data: None,
                request_duration_ms: duration,
                request_time,
            },
        }
    }

    /// Process a Gherkin step to build context
    fn process_step(
        &self,
        context: &mut StepContext,
        step: &GherkinStep,
        example_data: &serde_json::Value,
    ) {
        let text = self.substitute_placeholders(&step.text, example_data);

        // Handle doc string - prioritize this for request body
        if let Some(doc_str) = &step.doc_string {
            let substituted_doc = self.substitute_placeholders(doc_str, example_data);
            // Try to parse as JSON for request body
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&substituted_doc) {
                context.request_body = Some(json);
            }
        }

        // Handle data table - store for setup/validation
        if let Some(table_data) = &step.data_table {
            // Substitute placeholders in each row
            let processed_table: Vec<serde_json::Value> = table_data
                .iter()
                .map(|row| {
                    let row_str = row.to_string();
                    let substituted = self.substitute_placeholders(&row_str, example_data);
                    serde_json::from_str(&substituted).unwrap_or(row.clone())
                })
                .collect();
            context.setup_data = Some(processed_table);
        }

        // Parse common patterns from text
        if text.contains("request body") || text.contains("request payload") || text.contains("with body") {
            // Only set body from text if doc_string didn't already set it
            if context.request_body.is_none() {
                // Try to extract JSON from the step text
                if let Some(json_start) = text.find('{') {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text[json_start..]) {
                        context.request_body = Some(json);
                    }
                } else {
                    // Use example data as body if step mentions body
                    context.request_body = Some(example_data.clone());
                }
            }
        }

        if text.contains("header") {
            // Pattern: "I set header X-Custom-Header to value"
            self.parse_header(&text, context);
        }

        if text.contains("query param") || text.contains("query parameter") {
            // Pattern: "I set query param foo to bar"
            self.parse_query_param(&text, context);
        }

        // Check for status expectations in Then steps
        if step.keyword_type == "Outcome" {
            if let Some(status) = self.extract_status_code(&text) {
                context.expected_status = Some(status);
            }

            // Check for body contains
            if text.contains("contains") || text.contains("should have") {
                if let Some(pattern) = self.extract_quoted_string(&text) {
                    context.expected_body_contains.push(pattern);
                }
            }
        }
    }

    /// Substitute <placeholder> values with example data
    fn substitute_placeholders(&self, text: &str, example_data: &serde_json::Value) -> String {
        let mut result = text.to_string();

        if let Some(obj) = example_data.as_object() {
            for (key, value) in obj {
                let placeholder = format!("<{}>", key);
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => value.to_string(),
                };
                result = result.replace(&placeholder, &replacement);
            }
        }

        result
    }

    /// Build request body from example data
    fn build_request_body(
        &self,
        context: &StepContext,
        example_data: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        // If context has explicit body, use it
        if let Some(body) = &context.request_body {
            // Substitute placeholders in body
            let body_str = body.to_string();
            let substituted = self.substitute_placeholders(&body_str, example_data);
            serde_json::from_str(&substituted).ok()
        } else if !example_data.is_null() && example_data.is_object() {
            // Otherwise use example data directly (excluding expected_* fields)
            let mut body = example_data.clone();
            if let Some(obj) = body.as_object_mut() {
                obj.remove("expected_status");
                obj.remove("expected_status_code");
                obj.remove("expected_response_body");
            }
            Some(body)
        } else {
            None
        }
    }

    /// Execute HTTP request
    async fn execute_request(
        &self,
        api: &Api,
        environment: &Environment,
        context: &StepContext,
        example_data: &serde_json::Value,
    ) -> Result<(i16, serde_json::Value), AppError> {
        // Build URL
        let endpoint = self.substitute_placeholders(&api.endpoint, example_data);
        let mut url = format!("{}{}", environment.domain_name.trim_end_matches('/'), endpoint);

        // Add query params
        if !context.query_params.is_empty() {
            let params: Vec<String> = context
                .query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            url = format!("{}?{}", url, params.join("&"));
        }

        // Parse HTTP method
        let method = match api.http_method.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "PATCH" => Method::PATCH,
            "HEAD" => Method::HEAD,
            "OPTIONS" => Method::OPTIONS,
            _ => {
                return Err(AppError::Validation(format!(
                    "Unsupported HTTP method: {}",
                    api.http_method
                )))
            }
        };

        // Build request
        let mut request = self.client.request(method.clone(), &url);

        // Add auth header if configured
        if let Some(token) = &self.config.auth_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // Add custom headers from config
        for (key, value) in &self.config.custom_headers {
            request = request.header(key, value);
        }

        // Add headers from context
        for (key, value) in &context.request_headers {
            request = request.header(key, value);
        }

        // Add body for methods that support it
        if matches!(method, Method::POST | Method::PUT | Method::PATCH) {
            if let Some(body) = self.build_request_body(context, example_data) {
                request = request.json(&body);
            }
        }

        // Execute request
        let response: Response = request
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("HTTP request failed: {}", e)))?;

        let status = response.status().as_u16() as i16;

        // Parse response body
        let body: serde_json::Value = response
            .json()
            .await
            .unwrap_or(serde_json::Value::Null);

        Ok((status, body))
    }

    /// Validate response against expected values
    fn validate_response(
        &self,
        status: i16,
        body: &serde_json::Value,
        context: &StepContext,
    ) -> Result<(), String> {
        // Validate status code
        if let Some(expected_status) = context.expected_status {
            if status != expected_status {
                return Err(format!(
                    "Expected status {}, got {}",
                    expected_status, status
                ));
            }
        }

        // Validate body contains patterns
        for pattern in &context.expected_body_contains {
            let body_str = body.to_string();
            if !body_str.contains(pattern) {
                return Err(format!(
                    "Response body does not contain expected pattern: {}",
                    pattern
                ));
            }
        }

        // Validate expected body (deep comparison)
        if let Some(expected_body) = &context.expected_body {
            if !expected_body.is_null() {
                if !self.json_contains(body, expected_body) {
                    return Err(format!(
                        "Response body does not match expected. Expected: {}, Got: {}",
                        expected_body, body
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check if actual JSON contains all fields from expected JSON
    fn json_contains(&self, actual: &serde_json::Value, expected: &serde_json::Value) -> bool {
        match (actual, expected) {
            (serde_json::Value::Object(actual_obj), serde_json::Value::Object(expected_obj)) => {
                expected_obj.iter().all(|(key, expected_value)| {
                    actual_obj
                        .get(key)
                        .map(|actual_value| self.json_contains(actual_value, expected_value))
                        .unwrap_or(false)
                })
            }
            (serde_json::Value::Array(actual_arr), serde_json::Value::Array(expected_arr)) => {
                expected_arr.iter().all(|expected_item| {
                    actual_arr
                        .iter()
                        .any(|actual_item| self.json_contains(actual_item, expected_item))
                })
            }
            _ => actual == expected,
        }
    }

    /// Parse header from step text
    fn parse_header(&self, text: &str, context: &mut StepContext) {
        // Pattern: "header X-Custom-Header to value" or "header 'X-Custom-Header' with value 'foo'"
        let words: Vec<&str> = text.split_whitespace().collect();
        if let Some(header_idx) = words.iter().position(|&w| w == "header") {
            if let (Some(key), Some(value)) = (words.get(header_idx + 1), words.last()) {
                let key = key.trim_matches(|c| c == '\'' || c == '"');
                let value = value.trim_matches(|c| c == '\'' || c == '"');
                context
                    .request_headers
                    .insert(key.to_string(), value.to_string());
            }
        }
    }

    /// Parse query param from step text
    fn parse_query_param(&self, text: &str, context: &mut StepContext) {
        let words: Vec<&str> = text.split_whitespace().collect();
        if let Some(param_idx) = words.iter().position(|&w| w == "param" || w == "parameter") {
            if let (Some(key), Some(value)) = (words.get(param_idx + 1), words.last()) {
                let key = key.trim_matches(|c| c == '\'' || c == '"');
                let value = value.trim_matches(|c| c == '\'' || c == '"');
                context
                    .query_params
                    .insert(key.to_string(), value.to_string());
            }
        }
    }

    /// Extract status code from step text
    fn extract_status_code(&self, text: &str) -> Option<i16> {
        // Look for patterns like "status 200", "status code 200", "200 status"
        let words: Vec<&str> = text.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if let Ok(status) = word.parse::<i16>() {
                if status >= 100 && status < 600 {
                    return Some(status);
                }
            }
            // Check next word after "status"
            if *word == "status" || *word == "code" {
                if let Some(next) = words.get(i + 1) {
                    if let Ok(status) = next.parse::<i16>() {
                        if status >= 100 && status < 600 {
                            return Some(status);
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract quoted string from text
    fn extract_quoted_string(&self, text: &str) -> Option<String> {
        // Look for single or double quoted strings
        let mut in_quote = false;
        let mut quote_char = '"';
        let mut result = String::new();

        for c in text.chars() {
            if !in_quote && (c == '"' || c == '\'') {
                in_quote = true;
                quote_char = c;
            } else if in_quote && c == quote_char {
                return Some(result);
            } else if in_quote {
                result.push(c);
            }
        }

        None
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_placeholders() {
        let runner = TestRunner::new();
        let example = serde_json::json!({
            "email": "test@example.com",
            "password": "secret123"
        });

        let result = runner.substitute_placeholders("user <email> with password <password>", &example);
        assert_eq!(result, "user test@example.com with password secret123");
    }

    #[test]
    fn test_extract_status_code() {
        let runner = TestRunner::new();

        assert_eq!(runner.extract_status_code("status should be 200"), Some(200));
        assert_eq!(runner.extract_status_code("the status code is 404"), Some(404));
        assert_eq!(runner.extract_status_code("expect 500 error"), Some(500));
    }

    #[test]
    fn test_json_contains() {
        let runner = TestRunner::new();

        let actual = serde_json::json!({
            "id": 1,
            "name": "test",
            "extra": "field"
        });

        let expected = serde_json::json!({
            "id": 1,
            "name": "test"
        });

        assert!(runner.json_contains(&actual, &expected));

        let not_expected = serde_json::json!({
            "id": 2
        });
        assert!(!runner.json_contains(&actual, &not_expected));
    }
}
