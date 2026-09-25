use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminUser {
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RoleUpdate {
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct ActiveUpdate {
    pub is_active: bool,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct PlatformSettings {
    pub maintenance: bool,
    pub signups: bool,
    pub payments: bool,
    pub ai_recommendations: bool,
    pub public_teacher_directory: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub target_id: Option<Uuid>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DashboardMetrics {
    pub total_users: i64,
    pub new_users_30d: i64,
    pub active_tutors: i64,
    pub bookings_30d: i64,
    pub gmv_30d: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TutorVerification {
    pub user_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub county: Option<String>,
    pub estate: Option<String>,
    pub subjects: Vec<String>,
    pub photo_url: Option<String>,
    pub intro_video_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TutorVerificationUpdate {
    pub is_verified: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CentreSubmission {
    pub id: Uuid,
    pub name: String,
    pub county: String,
    pub estate: String,
    pub subjects: Vec<String>,
    pub monthly_fee: i32,
    pub owner_name: String,
    pub owner_email: String,
    pub created_at: DateTime<Utc>,
}
