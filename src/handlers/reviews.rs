use crate::middleware::auth::AuthUser;
use crate::models::reviews::{CreateReview, TutorReview};
use actix_web::{HttpResponse, Responder, get, post, web};
use sqlx::PgPool;
use uuid::Uuid;

#[get("/tutors/{tutor_id}/reviews")]
pub async fn list_reviews(pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let result = sqlx::query_as::<_, TutorReview>("SELECT r.id,r.rating,r.review,CONCAT(u.first_name,' ',u.last_name) AS reviewer_name,r.created_at FROM tutor_reviews r JOIN users u ON u.id=r.reviewer_id WHERE r.tutor_id=$1 ORDER BY r.created_at DESC LIMIT 20").bind(path.into_inner()).fetch_all(pool.get_ref()).await;
    match result {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/reviews")]
pub async fn create_review(
    user: AuthUser,
    pool: web::Data<PgPool>,
    body: web::Json<CreateReview>,
) -> impl Responder {
    if !matches!(user.role.as_str(), "student" | "parent") {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"message":"Only learners can review tutors"}));
    }
    let reviewer_id = match Uuid::parse_str(&user.user_id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::Unauthorized().finish(),
    };
    if !(1..=5).contains(&body.rating) || body.review.trim().len() < 10 {
        return HttpResponse::BadRequest().json(serde_json::json!({"message":"Provide a rating and a review of at least 10 characters"}));
    }
    let result = sqlx::query("INSERT INTO tutor_reviews (tutor_id,reviewer_id,booking_id,rating,review) SELECT b.tutor_id,$1,b.id,$2,$3 FROM tutor_bookings b WHERE b.id=$4 AND b.student_id=$1 AND b.status='completed'")
        .bind(reviewer_id).bind(body.rating).bind(body.review.trim()).bind(body.booking_id).execute(pool.get_ref()).await;
    match result {
        Ok(value) if value.rows_affected() == 1 => {
            HttpResponse::Created().json(serde_json::json!({"message":"Review submitted"}))
        }
        Ok(_) => HttpResponse::BadRequest()
            .json(serde_json::json!({"message":"Only a completed lesson can be reviewed"})),
        Err(sqlx::Error::Database(error)) if error.constraint().is_some() => {
            HttpResponse::Conflict()
                .json(serde_json::json!({"message":"This lesson has already been reviewed"}))
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
