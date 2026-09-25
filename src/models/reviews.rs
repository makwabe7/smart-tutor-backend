use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TutorReview {
    pub id: Uuid,
    pub rating: i16,
    pub review: String,
    pub reviewer_name: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Deserialize)]
pub struct CreateReview {
    pub booking_id: Uuid,
    pub rating: i16,
    pub review: String,
}
