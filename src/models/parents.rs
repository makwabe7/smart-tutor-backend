use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AddManagedChild {
    pub child_name: String,
    pub grade_level: Option<String>,
    pub relationship: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RequestStudentLink {
    pub student_email: String,
    pub relationship: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ParentStudent {
    pub id: Uuid,
    pub child_name: String,
    pub grade_level: Option<String>,
    pub relationship: String,
    pub status: String,
    pub student_id: Option<Uuid>,
    pub booking_count: i64,
    pub last_booking_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LinkRequest {
    pub id: Uuid,
    pub parent_name: String,
    pub relationship: String,
    pub created_at: DateTime<Utc>,
}
