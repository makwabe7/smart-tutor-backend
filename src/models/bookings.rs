use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateBooking {
    pub tutor_id: Uuid,
    pub availability_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub session_type: String,
    pub subject: String,
    pub grade_level: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BookingResponse {
    pub id: Uuid,
    pub teacher_id: Uuid,
    pub student_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BookingDetail {
    pub id: Uuid,
    pub teacher_id: Uuid,
    pub student_id: Uuid,
    pub teacher_name: String,
    pub student_name: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub session_type: String,
    pub subject: String,
    pub grade_level: Option<String>,
    pub total_amount: i32,
    pub status: String,
    pub notes: Option<String>,
    pub meeting_url: Option<String>,
}
