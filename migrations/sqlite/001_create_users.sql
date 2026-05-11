-- Users table
-- Note: UUIDs are TEXT in SQLite and produced by the application layer
-- (uuid::Uuid::new_v4()). SQLite has no native UUID type and no
-- equivalent to pgcrypto's gen_random_uuid().
--
-- KNOWN ISSUE: SeaORM currently writes `Uuid` columns to SQLite as BLOB
-- regardless of this `TEXT` declaration. Reads through the ORM round-trip
-- correctly, but `sqlite3` CLI queries on UUID columns return binary
-- noise. See CHANGELOG ([Unreleased] -> Known limitations) for the plan.
CREATE TABLE users (
    id            TEXT    PRIMARY KEY,
    email         TEXT    NOT NULL UNIQUE,
    password_hash TEXT    NOT NULL,
    name          TEXT    NOT NULL,
    job_title     TEXT,
    role          INTEGER NOT NULL DEFAULT 2,  -- 2 = standard user
    created_at    TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_users_email ON users(email);
