-- Environments table (dev, staging, production, etc.)
CREATE TABLE environments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title VARCHAR(100) NOT NULL,        -- e.g., "dev", "staging", "production"
    domain_name VARCHAR(500) NOT NULL,  -- base URL for this environment
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for project's environments lookup
CREATE INDEX idx_environments_project_id ON environments(project_id);

-- Unique constraint: title per project
CREATE UNIQUE INDEX idx_environments_project_title ON environments(project_id, title);
