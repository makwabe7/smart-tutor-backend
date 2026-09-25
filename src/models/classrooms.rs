use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Classroom {
    pub id: Uuid,
    pub booking_id: Uuid,
    pub title: String,
    pub room_id: String,
    pub provider: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub subject: String,
    pub grade_level: Option<String>,
    pub tutor_name: String,
    pub student_name: String,
}
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ClassroomMessage {
    pub id: Uuid,
    pub sender_id: Uuid,
    pub sender_name: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ClassroomResource {
    pub id: Uuid,
    pub uploaded_by: Uuid,
    pub name: String,
    pub file_url: String,
    pub resource_type: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Attendance {
    pub user_id: Uuid,
    pub user_name: String,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    pub left_at: Option<DateTime<Utc>>,
    pub duration_seconds: i32,
}
#[derive(Debug, Deserialize)]
pub struct SendMessage {
    pub message: String,
}
#[derive(Debug, Deserialize)]
pub struct CreateResource {
    pub name: String,
    pub file_url: String,
    pub resource_type: String,
}
#[derive(Debug, Serialize)]
pub struct ClassroomJoin {
    pub role: String,
    pub provider: String,
    pub livekit_url: Option<String>,
    pub media_token: Option<String>,
    pub zoom_url: Option<String>,
    pub expires_in_seconds: i64,
}
