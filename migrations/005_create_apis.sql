-- APIs table
CREATE TABLE apis (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    collection_id UUID NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    http_method VARCHAR(10) NOT NULL,   -- GET, POST, PUT, DELETE, PATCH, etc.
    endpoint VARCHAR(500) NOT NULL,     -- API endpoint path
    severity SMALLINT NOT NULL DEFAULT 1,  -- risk/severity level
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for collection's APIs lookup
CREATE INDEX idx_apis_collection_id ON apis(collection_id);

-- Unique constraint: API name per collection
CREATE UNIQUE INDEX idx_apis_collection_name ON apis(collection_id, name);

-- Index for http_method filtering
CREATE INDEX idx_apis_http_method ON apis(http_method);
