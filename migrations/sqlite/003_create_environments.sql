-- Environments table (dev, staging, production, etc.)
CREATE TABLE environments (
    id          TEXT NOT NULL PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,        -- e.g., "dev", "staging", "production"
    domain_name TEXT NOT NULL,        -- base URL for this environment
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_environments_project_id ON environments(project_id);
CREATE UNIQUE INDEX idx_environments_project_title ON environments(project_id, title);
