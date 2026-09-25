use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct UpdateTeacherProfile {
    pub county: Option<String>,
    pub estate: Option<String>,
    pub education_level: Option<String>,
    pub subjects: Vec<String>,
    pub grades: Vec<String>,
    pub years_experience: i32,
    pub hourly_rate: i32,
    pub bio: Option<String>,
    pub photo_url: Option<String>,
    pub intro_video_url: Option<String>,
    pub can_travel: bool,
    pub travel_radius_km: Option<i32>,
    pub availability: Vec<AvailabilityWindow>,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct AvailabilityWindow {
    pub weekday: i16,
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BookedSlot {
    pub scheduled_date: NaiveDate,
    pub scheduled_time: String,
    pub duration_minutes: i32,
    pub status: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TutorListing {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub county: Option<String>,
    pub estate: Option<String>,
    pub education_level: Option<String>,
    pub subjects: Vec<String>,
    pub grades: Vec<String>,
    pub years_experience: i32,
    pub hourly_rate: i32,
    pub bio: Option<String>,
    pub photo_url: Option<String>,
    pub intro_video_url: Option<String>,
    pub is_verified: bool,
    pub can_travel: bool,
    pub travel_radius_km: Option<i32>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TeacherBooking {
    pub id: Uuid,
    pub student_name: String,
    pub scheduled_date: NaiveDate,
    pub scheduled_time: String,
    pub mode: String,
    pub duration_minutes: i32,
    pub total_amount: i32,
    pub status: String,
}
