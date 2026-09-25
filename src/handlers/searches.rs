use actix_web::{HttpResponse, Responder, get, post, web};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::middleware::roles::Student;
use crate::models::teachers::TutorListing;

#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
}

fn student_id(student: &Student) -> Result<Uuid, HttpResponse> {
    Uuid::parse_str(&student.0.user_id).map_err(|_| {
        HttpResponse::Unauthorized().json(serde_json::json!({"message": "Invalid user ID"}))
    })
}

#[post("/searches")]
pub async fn record_search(
    student: Student,
    pool: web::Data<PgPool>,
    data: web::Json<SearchRequest>,
) -> impl Responder {
    let student_id = match student_id(&student) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let query = data.query.trim();
    if query.len() < 2 || query.len() > 200 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message": "Enter at least two characters"}));
    }
    match sqlx::query("INSERT INTO student_tutor_searches (student_id, query) VALUES ($1, $2)")
        .bind(student_id)
        .bind(query)
        .execute(pool.get_ref())
        .await
    {
        Ok(_) => HttpResponse::Created().finish(),
        Err(error) => {
            eprintln!("Search record error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/recommendations")]
pub async fn recommendations(student: Student, pool: web::Data<PgPool>) -> impl Responder {
    let student_id = match student_id(&student) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let result = sqlx::query_as::<_, TutorListing>(
        r#"SELECT DISTINCT ON (u.id) u.id, u.first_name, u.last_name, tp.county, tp.estate, tp.education_level, tp.subjects, tp.grades,
           tp.years_experience, tp.hourly_rate, tp.bio, tp.photo_url, tp.intro_video_url, tp.is_verified, tp.can_travel, tp.travel_radius_km
           FROM teacher_profiles tp JOIN users u ON u.id = tp.user_id
           JOIN student_tutor_searches s ON s.student_id = $1
           WHERE u.is_active = TRUE AND (lower(array_to_string(tp.subjects, ' ')) LIKE '%' || lower(s.query) || '%'
             OR lower(COALESCE(tp.estate, '')) LIKE '%' || lower(s.query) || '%')
           ORDER BY u.id, tp.is_verified DESC, tp.created_at DESC"#,
    ).bind(student_id).fetch_all(pool.get_ref()).await;
    match result {
        Ok(tutors) => HttpResponse::Ok().json(tutors),
        Err(error) => {
            eprintln!("Recommendations error: {error:?}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"message": "Could not load recommendations"}))
        }
    }
}
