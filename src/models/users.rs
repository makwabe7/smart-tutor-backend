use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

pub struct User {
    pub id: Uuid,
    pub email: String,
    pub phone: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub is_verified: bool,
    pub is_active: bool,
}
