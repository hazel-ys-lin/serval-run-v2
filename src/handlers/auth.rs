use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middlewares::AuthUser;
use crate::models::{CreateUser, UserResponse};
use crate::repositories::{RefreshTokenRepository, Repository, UserRepository};
use crate::services::AuthService;
use crate::state::AppState;

// ============ Request/Response DTOs ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub name: String,
    pub job_title: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: UserResponse,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub job_title: Option<String>,
}

// ============ Helpers ============

/// Create a refresh token in the DB and return the token string
async fn issue_refresh_token(
    state: &AppState,
    user_id: Uuid,
    family_id: Uuid,
) -> AppResult<String> {
    let token = AuthService::generate_refresh_token();
    let expires_at = OffsetDateTime::now_utc()
        + time::Duration::days(state.config.refresh_token_expiration_days);

    RefreshTokenRepository::create(&state.db, user_id, family_id, &token, expires_at).await?;

    Ok(token)
}

// ============ Handlers ============

/// Register a new user
#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = AuthResponse),
        (status = 409, description = "Email already exists"),
        (status = 400, description = "Validation error")
    ),
    tag = "Auth"
)]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    use crate::handlers::{validate_optional, validate_required};
    validate_required(&payload.email, "Email", 255)?;
    if !payload.email.contains('@') || !payload.email.contains('.') {
        return Err(AppError::Validation("Invalid email format".to_string()));
    }
    validate_required(&payload.password, "Password", 128)?;
    if payload.password.len() < 8 {
        return Err(AppError::Validation(
            "Password must be at least 8 characters".to_string(),
        ));
    }
    validate_required(&payload.name, "Name", 100)?;
    validate_optional(&payload.job_title, "Job title", 100)?;

    let password_hash = AuthService::hash_password(&payload.password)?;

    let create_user = CreateUser {
        email: payload.email,
        password: payload.password.clone(),
        name: payload.name,
        job_title: payload.job_title,
    };

    let user = UserRepository::create(&state.db, &create_user, &password_hash).await?;

    let token = AuthService::generate_token(user.id, &user.email, &state.config)?;
    let family_id = Uuid::new_v4();
    let refresh_token = issue_refresh_token(&state, user.id, family_id).await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token,
        user: user.into(),
    }))
}

/// Login with email and password
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = AuthResponse),
        (status = 401, description = "Invalid credentials")
    ),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let user = UserRepository::find_by_email(&state.db, &payload.email)
        .await
        .map_err(|_| AppError::InvalidCredentials)?;

    let is_valid = AuthService::verify_password(&payload.password, &user.password_hash)?;
    if !is_valid {
        return Err(AppError::InvalidCredentials);
    }

    let token = AuthService::generate_token(user.id, &user.email, &state.config)?;
    let family_id = Uuid::new_v4();
    let refresh_token = issue_refresh_token(&state, user.id, family_id).await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token,
        user: user.into(),
    }))
}

/// Exchange a refresh token for a new token pair (with rotation)
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = AuthResponse),
        (status = 401, description = "Invalid or expired refresh token")
    ),
    tag = "Auth"
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> AppResult<Json<AuthResponse>> {
    let stored = RefreshTokenRepository::find_by_token(&state.db, &payload.refresh_token)
        .await?
        .ok_or(AppError::InvalidToken)?;

    // Reuse detection: token exists but already revoked
    if stored.is_revoked() {
        // Someone is replaying an old token — nuke the entire session family
        tracing::warn!(
            family_id = %stored.family_id,
            user_id = %stored.user_id,
            "Refresh token reuse detected — revoking entire family"
        );
        RefreshTokenRepository::revoke_family(&state.db, stored.family_id).await?;
        return Err(AppError::Unauthorized);
    }

    // Expired but not revoked
    if stored.expires_at <= OffsetDateTime::now_utc() {
        return Err(AppError::TokenExpired);
    }

    // Rotate: revoke old token, issue new pair under same family
    RefreshTokenRepository::revoke(&state.db, stored.id).await?;

    let user = UserRepository::find_by_id(&state.db, stored.user_id).await?;
    let token = AuthService::generate_token(user.id, &user.email, &state.config)?;
    let new_refresh_token = issue_refresh_token(&state, user.id, stored.family_id).await?;

    Ok(Json(AuthResponse {
        token,
        refresh_token: new_refresh_token,
        user: user.into(),
    }))
}

/// Logout: revoke the provided refresh token
#[utoipa::path(
    post,
    path = "/api/auth/logout",
    request_body = LogoutRequest,
    responses(
        (status = 200, description = "Logged out successfully"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Auth"
)]
pub async fn logout(
    _user: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<LogoutRequest>,
) -> AppResult<StatusCode> {
    if let Some(stored) =
        RefreshTokenRepository::find_by_token(&state.db, &payload.refresh_token).await?
    {
        if !stored.is_revoked() {
            RefreshTokenRepository::revoke(&state.db, stored.id).await?;
        }
    }
    // Always return 200 — don't leak whether the token existed
    Ok(StatusCode::OK)
}

/// Get current authenticated user
#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses(
        (status = 200, description = "Current user info", body = UserResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Auth"
)]
pub async fn me(user: AuthUser, State(state): State<AppState>) -> AppResult<Json<UserResponse>> {
    let user_data = UserRepository::find_by_id(&state.db, user.id).await?;
    Ok(Json(user_data.into()))
}

/// Update current authenticated user
#[utoipa::path(
    put,
    path = "/api/users/me",
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = UserResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "Users"
)]
pub async fn update_me(
    user: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateUserRequest>,
) -> AppResult<Json<UserResponse>> {
    use crate::handlers::validate_optional;
    validate_optional(&payload.name, "Name", 100)?;
    validate_optional(&payload.job_title, "Job title", 100)?;

    let update_user = crate::models::UpdateUser {
        name: payload.name,
        job_title: payload.job_title,
    };

    let updated_user = UserRepository::update(&state.db, user.id, &update_user).await?;
    Ok(Json(updated_user.into()))
}
