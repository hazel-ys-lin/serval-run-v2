use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::{AppError, AppResult};
use crate::middlewares::AuthUser;
use crate::models::{CreateUser, UserResponse};
use crate::repositories::{Repository, UserRepository};
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

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub job_title: Option<String>,
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
    // Validate input
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

    // Hash password
    let password_hash = AuthService::hash_password(&payload.password)?;

    // Create user
    let create_user = CreateUser {
        email: payload.email,
        password: payload.password.clone(),
        name: payload.name,
        job_title: payload.job_title,
    };

    let user = UserRepository::create(&state.db, &create_user, &password_hash).await?;

    // Generate token
    let token = AuthService::generate_token(user.id, &user.email, &state.config)?;

    Ok(Json(AuthResponse {
        token,
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
    // Find user by email
    let user = UserRepository::find_by_email(&state.db, &payload.email)
        .await
        .map_err(|_| AppError::InvalidCredentials)?;

    // Verify password
    let is_valid = AuthService::verify_password(&payload.password, &user.password_hash)?;
    if !is_valid {
        return Err(AppError::InvalidCredentials);
    }

    // Generate token
    let token = AuthService::generate_token(user.id, &user.email, &state.config)?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
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
