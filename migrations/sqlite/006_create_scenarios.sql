-- Scenarios table (test scenarios for APIs)
-- tags/steps/examples are TEXT columns holding JSON. SQLite has no array
-- or JSONB type, and no GIN-equivalent for indexing inside JSON payloads
-- — search on those fields, if needed, will run at the application layer.
CREATE TABLE scenarios (
    id          TEXT NOT NULL PRIMARY KEY,
    api_id      TEXT NOT NULL REFERENCES apis(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    description TEXT,
    tags        TEXT NOT NULL DEFAULT '[]',  -- JSON array of strings
    steps       TEXT NOT NULL DEFAULT '[]',  -- JSON: [{keyword, keywordType, text}]
    examples    TEXT NOT NULL DEFAULT '[]',  -- JSON: [{example, expected_response_body, expected_status_code}]
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_scenarios_api_id ON scenarios(api_id);
