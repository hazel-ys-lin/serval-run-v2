-- Reports table (test execution reports)
CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    environment_id UUID NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    collection_id UUID REFERENCES collections(id) ON DELETE SET NULL,  -- nullable for project-level reports

    -- Report metadata
    report_level SMALLINT NOT NULL DEFAULT 1,  -- 1 = collection, 2 = project
    report_type VARCHAR(50),                   -- type of test run

    -- Status and results
    finished BOOLEAN NOT NULL DEFAULT FALSE,
    calculated BOOLEAN NOT NULL DEFAULT FALSE,
    pass_rate DECIMAL(5, 2),                   -- percentage (0.00 - 100.00)
    response_count INTEGER NOT NULL DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ
);

-- Indexes for common queries
CREATE INDEX idx_reports_project_id ON reports(project_id);
CREATE INDEX idx_reports_environment_id ON reports(environment_id);
CREATE INDEX idx_reports_collection_id ON reports(collection_id);
CREATE INDEX idx_reports_created_at ON reports(created_at DESC);
CREATE INDEX idx_reports_finished ON reports(finished);
