-- Collections table (groups of related APIs)
CREATE TABLE collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for project's collections lookup
CREATE INDEX idx_collections_project_id ON collections(project_id);

-- Unique constraint: collection name per project
CREATE UNIQUE INDEX idx_collections_project_name ON collections(project_id, name);
