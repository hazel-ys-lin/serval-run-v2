-- Refresh tokens table for token rotation and reuse detection
CREATE TABLE refresh_tokens (
    id         TEXT NOT NULL PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token      TEXT NOT NULL UNIQUE,             -- UUID v4 raw string
    family_id  TEXT NOT NULL,                    -- shared across rotation chain; used for reuse detection
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TEXT                              -- NULL = active; non-NULL = revoked
);

CREATE INDEX idx_refresh_tokens_token   ON refresh_tokens(token);
CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_family  ON refresh_tokens(family_id);
