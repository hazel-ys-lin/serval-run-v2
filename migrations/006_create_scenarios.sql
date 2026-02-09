-- Scenarios table (test scenarios for APIs)
CREATE TABLE scenarios (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    api_id UUID NOT NULL REFERENCES apis(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    tags TEXT[] DEFAULT '{}',           -- array of tags for categorization
    steps JSONB NOT NULL DEFAULT '[]',  -- Gherkin steps: [{keyword, keywordType, text}]
    examples JSONB NOT NULL DEFAULT '[]', -- test data: [{example, expected_response_body, expected_status_code}]
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for API's scenarios lookup
CREATE INDEX idx_scenarios_api_id ON scenarios(api_id);

-- GIN index for tags array search
CREATE INDEX idx_scenarios_tags ON scenarios USING GIN(tags);

-- GIN index for JSONB queries on steps and examples
CREATE INDEX idx_scenarios_steps ON scenarios USING GIN(steps);
CREATE INDEX idx_scenarios_examples ON scenarios USING GIN(examples);
