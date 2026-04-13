use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub family_id: Uuid,
    pub expires_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
    pub revoked_at: Option<OffsetDateTime>,
}

impl RefreshToken {
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none() && self.expires_at > OffsetDateTime::now_utc()
    }

    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }
}
