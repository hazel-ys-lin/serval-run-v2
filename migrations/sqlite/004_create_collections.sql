-- Collections table (groups of related APIs)
CREATE TABLE collections (
    id          TEXT NOT NULL PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_collections_project_id ON collections(project_id);
CREATE UNIQUE INDEX idx_collections_project_name ON collections(project_id, name);
