use actix_web::{HttpResponse, Responder, get, put, web};
use sqlx::PgPool;
use uuid::Uuid;

use crate::middleware::roles::Student;
use crate::models::students::{StudentProfile, UpdateStudentProfile};

/// GET /api/student/dashboard
#[get("/dashboard")]
pub async fn dashboard(_student: Student) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Welcome to Student Dashboard!"
    }))
}

/// GET /api/student/profile
#[get("/profile")]
pub async fn get_profile(student: Student, pool: web::Data<PgPool>) -> impl Responder {
    // Convert user ID from JWT string to UUID
    let user_id = match Uuid::parse_str(&student.0.user_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "message": "Invalid user ID"
            }));
        }
    };

    // Find student profile
    let profile = sqlx::query_as::<_, StudentProfile>(
        r#"
        SELECT
            id,
            user_id,
            date_of_birth,
            gender,
            school_name,
            grade_level,
            county,
            bio
        FROM student_profiles
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await;

    match profile {
        // Profile exists
        Ok(Some(profile)) => HttpResponse::Ok().json(profile),

        // Profile does not exist
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "message": "Student profile not found"
        })),

        // Database error
        Err(error) => {
            eprintln!("Student profile error: {:?}", error);

            HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Database error"
            }))
        }
    }
}

/// PUT /api/student/profile
#[put("/profile")]
pub async fn update_profile(
    student: Student,
    pool: web::Data<PgPool>,
    data: web::Json<UpdateStudentProfile>,
) -> impl Responder {
    // Convert user ID from JWT string to UUID
    let user_id = match Uuid::parse_str(&student.0.user_id) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "message": "Invalid user ID"
            }));
        }
    };

    // Insert profile or update existing profile
    let profile = sqlx::query_as::<_, StudentProfile>(
        r#"
        INSERT INTO student_profiles (
            user_id,
            date_of_birth,
            gender,
            school_name,
            grade_level,
            county,
            bio
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)

        ON CONFLICT (user_id)
        DO UPDATE SET
            date_of_birth = EXCLUDED.date_of_birth,
            gender = EXCLUDED.gender,
            school_name = EXCLUDED.school_name,
            grade_level = EXCLUDED.grade_level,
            county = EXCLUDED.county,
            bio = EXCLUDED.bio,
            updated_at = NOW()

        RETURNING
            id,
            user_id,
            date_of_birth,
            gender,
            school_name,
            grade_level,
            county,
            bio
        "#,
    )
    .bind(user_id)
    .bind(data.date_of_birth)
    .bind(data.gender.as_deref())
    .bind(data.school_name.as_deref())
    .bind(data.grade_level.as_deref())
    .bind(data.county.as_deref())
    .bind(data.bio.as_deref())
    .fetch_one(pool.get_ref())
    .await;

    match profile {
        // Profile successfully created/updated
        Ok(profile) => HttpResponse::Ok().json(profile),

        // Database error
        Err(error) => {
            eprintln!("Student profile update error: {:?}", error);

            HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Failed to update student profile"
            }))
        }
    }
}
