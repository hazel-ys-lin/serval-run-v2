//! Database backend abstraction.
//!
//! v0.x of ServalRun supports two SQL backends:
//!
//! - **Postgres** for shared / team deployments. Pulls in the full
//!   docker-compose stack (postgres, mongodb, redis) and is what the
//!   v1 baseline (v0.1.0) ships with.
//! - **SQLite** for lite mode — single-engineer local use, CLI, and CI.
//!   Aims to remove the docker dependency entirely.
//!
//! This module owns the *choice* of backend. Connection setup itself
//! still lives in [`crate::state`]; that layer dispatches on the
//! [`DbBackend`] value returned from [`detect_backend`].
//!
//! The decision is driven by the URL scheme so `DATABASE_URL` is the
//! single source of truth — there is no separate `DB_KIND` env var to
//! drift out of sync.

use std::fmt;

/// Which SQL backend ServalRun should talk to.
///
/// New variants here will need matching arms in [`crate::state::AppState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbBackend {
    Postgres,
    Sqlite,
}

impl fmt::Display for DbBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbBackend::Postgres => f.write_str("postgres"),
            DbBackend::Sqlite => f.write_str("sqlite"),
        }
    }
}

/// Errors that can occur while parsing the database URL.
#[derive(Debug, thiserror::Error)]
pub enum DbBackendError {
    #[error("DATABASE_URL is empty")]
    Empty,

    #[error(
        "unsupported DATABASE_URL scheme {scheme:?}; expected one of \
         postgres://, postgresql://, sqlite://, sqlite:"
    )]
    UnsupportedScheme { scheme: String },
}

/// Infer the backend from a connection URL.
///
/// Accepted schemes:
/// - `postgres://...` / `postgresql://...`         → [`DbBackend::Postgres`]
/// - `sqlite://...` / `sqlite:...` / `sqlite::memory:` → [`DbBackend::Sqlite`]
///
/// Anything else returns [`DbBackendError::UnsupportedScheme`] so a typo
/// in DATABASE_URL fails fast at startup rather than at the first query.
pub fn detect_backend(url: &str) -> Result<DbBackend, DbBackendError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(DbBackendError::Empty);
    }

    // sqlite::memory: has no // separator; check it first.
    if trimmed.starts_with("sqlite:") {
        return Ok(DbBackend::Sqlite);
    }

    let scheme = trimmed.split("://").next().unwrap_or("");
    match scheme {
        "postgres" | "postgresql" => Ok(DbBackend::Postgres),
        other => Err(DbBackendError::UnsupportedScheme {
            scheme: other.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_postgres() {
        assert_eq!(
            detect_backend("postgres://user:pw@localhost/db").unwrap(),
            DbBackend::Postgres
        );
        assert_eq!(
            detect_backend("postgresql://user:pw@localhost/db").unwrap(),
            DbBackend::Postgres
        );
    }

    #[test]
    fn detects_sqlite_file() {
        assert_eq!(
            detect_backend("sqlite:///tmp/foo.db").unwrap(),
            DbBackend::Sqlite
        );
        assert_eq!(
            detect_backend("sqlite:./foo.db").unwrap(),
            DbBackend::Sqlite
        );
    }

    #[test]
    fn detects_sqlite_in_memory() {
        assert_eq!(
            detect_backend("sqlite::memory:").unwrap(),
            DbBackend::Sqlite
        );
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(detect_backend("   "), Err(DbBackendError::Empty)));
    }

    #[test]
    fn rejects_unknown_scheme() {
        let err = detect_backend("mysql://user@localhost/db").unwrap_err();
        match err {
            DbBackendError::UnsupportedScheme { scheme } => assert_eq!(scheme, "mysql"),
            _ => panic!("wrong error variant: {err:?}"),
        }
    }

    #[test]
    fn display_matches_scheme() {
        assert_eq!(DbBackend::Postgres.to_string(), "postgres");
        assert_eq!(DbBackend::Sqlite.to_string(), "sqlite");
    }
}
