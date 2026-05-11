-- Reports table (test execution reports)
CREATE TABLE reports (
    id             TEXT    NOT NULL PRIMARY KEY,
    project_id     TEXT    NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    environment_id TEXT    NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    collection_id  TEXT             REFERENCES collections(id) ON DELETE SET NULL,  -- nullable for project-level reports

    -- Report metadata
    report_level   INTEGER NOT NULL DEFAULT 1,  -- 1 = collection, 2 = project
    report_type    TEXT,                        -- type of test run

    -- Status and results
    finished       INTEGER NOT NULL DEFAULT 0,  -- 0/1 boolean
    calculated     INTEGER NOT NULL DEFAULT 0,  -- 0/1 boolean
    pass_rate      REAL,                        -- percentage (0.0 - 100.0)
    response_count INTEGER NOT NULL DEFAULT 0,

    created_at     TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at    TEXT
);

CREATE INDEX idx_reports_project_id     ON reports(project_id);
CREATE INDEX idx_reports_environment_id ON reports(environment_id);
CREATE INDEX idx_reports_collection_id  ON reports(collection_id);
CREATE INDEX idx_reports_created_at     ON reports(created_at DESC);
CREATE INDEX idx_reports_finished       ON reports(finished);
