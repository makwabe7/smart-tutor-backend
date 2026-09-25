use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub category: String,
    pub href: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}
