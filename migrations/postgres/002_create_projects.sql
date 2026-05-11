-- Projects table
CREATE TABLE projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for user's projects lookup
CREATE INDEX idx_projects_user_id ON projects(user_id);

-- Unique constraint: project name per user
CREATE UNIQUE INDEX idx_projects_user_name ON projects(user_id, name);
