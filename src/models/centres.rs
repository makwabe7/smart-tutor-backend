use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TuitionCentre {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub tagline: Option<String>,
    pub county: String,
    pub estate: String,
    pub address: Option<String>,
    pub description: Option<String>,
    pub subjects: Vec<String>,
    pub grade_levels: Vec<String>,
    pub monthly_fee: i32,
    pub student_count: i32,
    pub rating: f64,
    pub review_count: i32,
    pub image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTuitionCentre {
    pub name: String,
    pub tagline: Option<String>,
    pub county: String,
    pub estate: String,
    pub address: Option<String>,
    pub description: Option<String>,
    pub subjects: Vec<String>,
    pub grade_levels: Vec<String>,
    pub monthly_fee: i32,
    pub image_url: Option<String>,
}
