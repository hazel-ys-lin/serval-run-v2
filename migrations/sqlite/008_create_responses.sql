-- Responses table (individual test execution results)
CREATE TABLE responses (
    id                  TEXT    NOT NULL PRIMARY KEY,
    report_id           TEXT    NOT NULL REFERENCES reports(id)    ON DELETE CASCADE,
    api_id              TEXT    NOT NULL REFERENCES apis(id)       ON DELETE CASCADE,
    scenario_id         TEXT    NOT NULL REFERENCES scenarios(id)  ON DELETE CASCADE,
    example_index       INTEGER NOT NULL,                          -- index of the example in scenario.examples[]

    -- Actual response data
    response_data       TEXT,                                      -- JSON: actual response body
    response_status     INTEGER NOT NULL,                          -- HTTP status code

    -- Test result
    pass                INTEGER NOT NULL,                          -- 0/1 boolean
    error_message       TEXT,                                      -- error details if failed

    -- Timing
    request_time        TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    request_duration_ms INTEGER                                    -- request duration in milliseconds
);

CREATE INDEX idx_responses_report_id    ON responses(report_id);
CREATE INDEX idx_responses_api_id       ON responses(api_id);
CREATE INDEX idx_responses_scenario_id  ON responses(scenario_id);
CREATE INDEX idx_responses_pass         ON responses(pass);
CREATE INDEX idx_responses_report_pass  ON responses(report_id, pass);
