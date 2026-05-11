-- Responses table (individual test execution results)
CREATE TABLE responses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id UUID NOT NULL REFERENCES reports(id) ON DELETE CASCADE,
    api_id UUID NOT NULL REFERENCES apis(id) ON DELETE CASCADE,
    scenario_id UUID NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
    example_index INTEGER NOT NULL,          -- index of the example in scenario.examples[]

    -- Actual response data
    response_data JSONB,                     -- actual response body
    response_status SMALLINT NOT NULL,       -- HTTP status code

    -- Test result
    pass BOOLEAN NOT NULL,
    error_message TEXT,                      -- error details if failed

    -- Timing
    request_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    request_duration_ms INTEGER              -- request duration in milliseconds
);

-- Indexes for common queries
CREATE INDEX idx_responses_report_id ON responses(report_id);
CREATE INDEX idx_responses_api_id ON responses(api_id);
CREATE INDEX idx_responses_scenario_id ON responses(scenario_id);
CREATE INDEX idx_responses_pass ON responses(pass);

-- Composite index for report result queries
CREATE INDEX idx_responses_report_pass ON responses(report_id, pass);
