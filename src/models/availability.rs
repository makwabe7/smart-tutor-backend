use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AvailabilityInput {
    pub day_of_week: i16,
    // HTML time inputs submit HH:MM; parse and validate these in the handler.
    pub start_time: String,
    pub end_time: String,
    pub slot_duration_minutes: i32,
    pub session_type: String,
    pub subject: Option<String>,
    pub grade_level: Option<String>,
    pub available_from: NaiveDate,
    pub available_until: Option<NaiveDate>,
    pub repeats_weekly: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AvailabilityRule {
    pub id: Uuid,
    pub teacher_id: Uuid,
    pub day_of_week: i16,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub slot_duration_minutes: i32,
    pub session_type: String,
    pub subject: Option<String>,
    pub grade_level: Option<String>,
    pub available_from: NaiveDate,
    pub available_until: Option<NaiveDate>,
    pub repeats_weekly: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AvailableSlot {
    pub availability_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub session_type: String,
    pub subject: Option<String>,
    pub grade_level: Option<String>,
    pub slot_duration_minutes: i32,
}
