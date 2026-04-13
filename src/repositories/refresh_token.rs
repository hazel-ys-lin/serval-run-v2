use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::entity::refresh_token::{self, ActiveModel, Column, Entity as RefreshTokenEntity};
use crate::error::{AppError, AppResult};
use crate::models::RefreshToken;

pub struct RefreshTokenRepository;

impl RefreshTokenRepository {
    /// Create a new refresh token record
    pub async fn create(
        db: &DatabaseConnection,
        user_id: Uuid,
        family_id: Uuid,
        token: &str,
        expires_at: OffsetDateTime,
    ) -> AppResult<RefreshToken> {
        let model = ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            token: Set(token.to_string()),
            family_id: Set(family_id),
            expires_at: Set(expires_at),
            created_at: Set(OffsetDateTime::now_utc()),
            revoked_at: Set(None),
        };

        let result = model.insert(db).await.map_err(|e| {
            tracing::error!("Failed to create refresh token: {e}");
            AppError::Database("An unexpected database error occurred".to_string())
        })?;

        Ok(result.into())
    }

    /// Find a refresh token by its token string
    pub async fn find_by_token(
        db: &DatabaseConnection,
        token: &str,
    ) -> AppResult<Option<RefreshToken>> {
        let model = RefreshTokenEntity::find()
            .filter(Column::Token.eq(token))
            .one(db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find refresh token: {e}");
                AppError::Database("An unexpected database error occurred".to_string())
            })?;

        Ok(model.map(Into::into))
    }

    /// Revoke a single token by its DB id
    pub async fn revoke(db: &DatabaseConnection, id: Uuid) -> AppResult<()> {
        let model = RefreshTokenEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find refresh token for revocation: {e}");
                AppError::Database("An unexpected database error occurred".to_string())
            })?
            .ok_or_else(|| AppError::NotFound("RefreshToken".to_string()))?;

        let mut active: ActiveModel = model.into();
        active.revoked_at = Set(Some(OffsetDateTime::now_utc()));
        active.update(db).await.map_err(|e| {
            tracing::error!("Failed to revoke refresh token: {e}");
            AppError::Database("An unexpected database error occurred".to_string())
        })?;

        Ok(())
    }

    /// Revoke all tokens in a family (triggered on reuse detection)
    pub async fn revoke_family(db: &DatabaseConnection, family_id: Uuid) -> AppResult<()> {
        use sea_orm::{ActiveModelTrait, IntoActiveModel};

        let tokens = RefreshTokenEntity::find()
            .filter(Column::FamilyId.eq(family_id))
            .filter(Column::RevokedAt.is_null())
            .all(db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find refresh token family: {e}");
                AppError::Database("An unexpected database error occurred".to_string())
            })?;

        let now = OffsetDateTime::now_utc();
        for token in tokens {
            let mut active = token.into_active_model();
            active.revoked_at = Set(Some(now));
            active.update(db).await.map_err(|e| {
                tracing::error!("Failed to revoke token in family: {e}");
                AppError::Database("An unexpected database error occurred".to_string())
            })?;
        }

        Ok(())
    }

    /// Revoke all active tokens for a user (logout all sessions)
    pub async fn revoke_all_for_user(db: &DatabaseConnection, user_id: Uuid) -> AppResult<()> {
        use sea_orm::IntoActiveModel;

        let tokens = RefreshTokenEntity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::RevokedAt.is_null())
            .all(db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find user refresh tokens: {e}");
                AppError::Database("An unexpected database error occurred".to_string())
            })?;

        let now = OffsetDateTime::now_utc();
        for token in tokens {
            let mut active = token.into_active_model();
            active.revoked_at = Set(Some(now));
            active.update(db).await.map_err(|e| {
                tracing::error!("Failed to revoke user token: {e}");
                AppError::Database("An unexpected database error occurred".to_string())
            })?;
        }

        Ok(())
    }
}

impl From<refresh_token::Model> for RefreshToken {
    fn from(m: refresh_token::Model) -> Self {
        Self {
            id: m.id,
            user_id: m.user_id,
            token: m.token,
            family_id: m.family_id,
            expires_at: m.expires_at,
            created_at: m.created_at,
            revoked_at: m.revoked_at,
        }
    }
}
