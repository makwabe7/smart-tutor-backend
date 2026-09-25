use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StudentProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub date_of_birth: Option<NaiveDate>,
    pub gender: Option<String>,

    pub school_name: Option<String>,
    pub grade_level: Option<String>,
    pub county: Option<String>,
    pub bio: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct UpdateStudentProfile {
    pub date_of_birth: Option<NaiveDate>,
    pub gender: Option<String>,
    pub school_name: Option<String>,
    pub grade_level: Option<String>,
    pub county: Option<String>,
    pub bio: Option<String>,
}
