-- APIs table
CREATE TABLE apis (
    id            TEXT    NOT NULL PRIMARY KEY,
    collection_id TEXT    NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    name          TEXT    NOT NULL,
    http_method   TEXT    NOT NULL,            -- GET, POST, PUT, DELETE, PATCH, etc.
    endpoint      TEXT    NOT NULL,            -- API endpoint path
    severity      INTEGER NOT NULL DEFAULT 1,  -- risk/severity level
    description   TEXT,
    created_at    TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_apis_collection_id ON apis(collection_id);
CREATE UNIQUE INDEX idx_apis_collection_name ON apis(collection_id, name);
CREATE INDEX idx_apis_http_method ON apis(http_method);
