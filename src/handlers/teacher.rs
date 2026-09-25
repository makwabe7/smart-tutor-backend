use actix_web::{HttpResponse, Responder, get, post, put, web};
use chrono::{Duration, Utc};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use std::env;

use crate::middleware::auth::AuthUser;
use crate::models::teachers::{
    AvailabilityWindow, BookedSlot, TeacherBooking, TutorListing, UpdateTeacherProfile,
};

fn teacher_id(user: &AuthUser) -> Result<Uuid, HttpResponse> {
    if user.role != "teacher" {
        return Err(HttpResponse::Forbidden()
            .json(serde_json::json!({"message": "Teacher access required"})));
    }
    Uuid::parse_str(&user.user_id).map_err(|_| {
        HttpResponse::Unauthorized().json(serde_json::json!({"message": "Invalid user ID"}))
    })
}

#[derive(Deserialize)]
pub struct PayoutRequest {
    pub amount: i32,
}

#[derive(Deserialize)]
pub struct BookingStatusUpdate {
    pub status: String,
}

async fn wallet_summary(pool: &PgPool, tutor_id: Uuid) -> Result<(i32, i32, i32), sqlx::Error> {
    let confirmed = sqlx::query_scalar::<_, i64>("SELECT COALESCE(SUM(total_amount), 0) FROM tutor_bookings WHERE tutor_id=$1 AND status='confirmed'").bind(tutor_id).fetch_one(pool).await? as i32;
    let reserved = sqlx::query_scalar::<_, i64>("SELECT COALESCE(SUM(amount), 0) FROM teacher_payouts WHERE teacher_id=$1 AND status IN ('requested','processing','paid')").bind(tutor_id).fetch_one(pool).await? as i32;
    Ok((confirmed, reserved, (confirmed - reserved).max(0)))
}

#[get("/api/teacher/wallet")]
pub async fn wallet(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    let tutor_id = match teacher_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    match wallet_summary(pool.get_ref(), tutor_id).await {
        Ok((confirmed_earnings, paid_or_pending, available_balance)) => HttpResponse::Ok().json(serde_json::json!({"confirmed_earnings":confirmed_earnings,"paid_or_pending":paid_or_pending,"available_balance":available_balance})),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({"message":"Could not load wallet"})),
    }
}

#[post("/api/teacher/wallet/payouts")]
pub async fn request_payout(
    user: AuthUser,
    pool: web::Data<PgPool>,
    body: web::Json<PayoutRequest>,
) -> impl Responder {
    let tutor_id = match teacher_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let (_, _, available) = match wallet_summary(pool.get_ref(), tutor_id).await {
        Ok(value) => value,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    if body.amount <= 0 || body.amount > available {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message":"Enter an amount within your available balance"}));
    }
    let phone = sqlx::query_scalar::<_, Option<String>>("SELECT phone FROM users WHERE id=$1")
        .bind(tutor_id)
        .fetch_one(pool.get_ref())
        .await
        .ok()
        .flatten();
    let phone = match phone { Some(value) => value, None => return HttpResponse::BadRequest().json(serde_json::json!({"message":"Add a phone number to your profile before requesting a payout"})) };
    match sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO teacher_payouts (teacher_id, amount, phone) VALUES ($1,$2,$3) RETURNING id",
    )
    .bind(tutor_id)
    .bind(body.amount)
    .bind(phone)
    .fetch_one(pool.get_ref())
    .await
    {
        Ok(id) => {
            HttpResponse::Created().json(serde_json::json!({"id":id,"message":"Payout requested"}))
        }
        Err(_) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"message":"Could not request payout"})),
    }
}

#[put("/api/teacher/bookings/{id}/status")]
pub async fn update_booking_status(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<BookingStatusUpdate>,
) -> impl Responder {
    let tutor_id = match teacher_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    if !matches!(body.status.as_str(), "confirmed" | "cancelled") {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message":"Choose confirmed or cancelled"}));
    }
    let updated = sqlx::query_scalar::<_, Uuid>("UPDATE tutor_bookings SET status=$1 WHERE id=$2 AND tutor_id=$3 AND status='pending' RETURNING id")
        .bind(&body.status).bind(path.into_inner()).bind(tutor_id).fetch_optional(pool.get_ref()).await;
    match updated {
        Ok(Some(id)) => {
            if body.status == "confirmed" {
                let provider = env::var("CLASSROOM_PROVIDER").unwrap_or_else(|_| "internal".to_string());
                let room_id = if provider == "zoom" {
                    env::var("ZOOM_MEETING_URL").unwrap_or_else(|_| format!("soma-{}", id))
                } else { format!("soma-{}", id) };
                let classroom = sqlx::query(r#"
                    INSERT INTO classroom_sessions (booking_id,title,room_id,provider)
                    SELECT b.id, CONCAT(b.subject, COALESCE(CONCAT(' — ', b.grade_level), '')), $2, $3
                    FROM tutor_bookings b WHERE b.id=$1 AND b.session_type='online'
                    ON CONFLICT (booking_id) DO NOTHING
                "#).bind(id).bind(room_id).bind(provider).execute(pool.get_ref()).await;
                if classroom.is_err() {
                    return HttpResponse::InternalServerError().json(serde_json::json!({"message":"Booking confirmed but classroom creation failed"}));
                }
            }
            HttpResponse::Ok().json(serde_json::json!({"id":id,"status":body.status}))
        }
        Ok(None) => HttpResponse::NotFound()
            .json(serde_json::json!({"message":"Booking request is no longer pending"})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

/// Creates or updates the profile that appears in the public tutor catalogue.
#[put("/api/teacher/profile")]
pub async fn update_profile(
    user: AuthUser,
    pool: web::Data<PgPool>,
    data: web::Json<UpdateTeacherProfile>,
) -> impl Responder {
    let user_id = match teacher_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    if data.subjects.is_empty()
        || data.grades.is_empty()
        || data.hourly_rate < 0
        || data.years_experience < 0
    {
        return HttpResponse::BadRequest().json(
            serde_json::json!({"message": "Add subjects, grades, and valid teaching details"}),
        );
    }
    if data.can_travel && !matches!(data.travel_radius_km, Some(1..=100)) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message": "Set a travel radius between 1 and 100 km"}));
    }
    if data
        .availability
        .iter()
        .any(|slot| slot.weekday < 0 || slot.weekday > 6 || slot.start_time >= slot.end_time)
    {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message": "Add valid weekly availability times"}));
    }

    let result = sqlx::query(
        r#"INSERT INTO teacher_profiles
        (user_id, county, estate, education_level, subjects, grades, years_experience, hourly_rate, bio, photo_url, intro_video_url, can_travel, travel_radius_km)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
        ON CONFLICT (user_id) DO UPDATE SET
          county=EXCLUDED.county, estate=EXCLUDED.estate, education_level=EXCLUDED.education_level,
          subjects=EXCLUDED.subjects, grades=EXCLUDED.grades, years_experience=EXCLUDED.years_experience,
          hourly_rate=EXCLUDED.hourly_rate, bio=EXCLUDED.bio, photo_url=EXCLUDED.photo_url, intro_video_url=EXCLUDED.intro_video_url,
          can_travel=EXCLUDED.can_travel, travel_radius_km=EXCLUDED.travel_radius_km, updated_at=NOW()"#,
    )
    .bind(user_id).bind(&data.county).bind(&data.estate).bind(&data.education_level)
    .bind(&data.subjects).bind(&data.grades).bind(data.years_experience).bind(data.hourly_rate)
    .bind(&data.bio).bind(&data.photo_url).bind(&data.intro_video_url).bind(data.can_travel).bind(data.travel_radius_km)
    .execute(pool.get_ref()).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({"message": "Teaching profile saved"})),
        Err(error) => {
            eprintln!("Teacher profile update error: {error:?}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"message": "Could not save teaching profile"}))
        }
    }
}

#[get("/api/teacher/dashboard")]
pub async fn dashboard(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    let tutor_id = match teacher_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let profile = sqlx::query_as::<_, TutorListing>(
        "SELECT u.id, u.first_name, u.last_name, tp.county, tp.estate, tp.education_level, tp.subjects, tp.grades, tp.years_experience, tp.hourly_rate, tp.bio, tp.photo_url, tp.intro_video_url, tp.is_verified, tp.can_travel, tp.travel_radius_km FROM teacher_profiles tp JOIN users u ON u.id = tp.user_id WHERE tp.user_id = $1",
    ).bind(tutor_id).fetch_optional(pool.get_ref()).await;
    let profile = match profile {
        Ok(Some(profile)) => profile,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({"message": "Complete your teaching profile first"}));
        }
        Err(error) => {
            eprintln!("Teacher dashboard profile error: {error:?}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let availability = match sqlx::query_as::<_, AvailabilityWindow>("SELECT weekday, start_time::text AS start_time, end_time::text AS end_time FROM teacher_availability WHERE teacher_id=$1 ORDER BY weekday")
        .bind(tutor_id).fetch_all(pool.get_ref()).await {
        Ok(value) => value,
        Err(error) => { eprintln!("Teacher dashboard availability error: {error:?}"); return HttpResponse::InternalServerError().finish(); }
    };
    let bookings = sqlx::query_as::<_, TeacherBooking>(
        "SELECT b.id, CONCAT(s.first_name, ' ', s.last_name) AS student_name, b.scheduled_date, b.scheduled_time, b.mode, b.duration_minutes, b.total_amount, b.status FROM tutor_bookings b JOIN users s ON s.id = b.student_id WHERE b.tutor_id = $1 ORDER BY b.scheduled_date ASC, b.scheduled_time ASC",
    ).bind(tutor_id).fetch_all(pool.get_ref()).await;
    let bookings = match bookings {
        Ok(rows) => rows,
        Err(error) => {
            eprintln!("Teacher dashboard bookings error: {error:?}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    let pending = bookings.iter().filter(|b| b.status == "pending").count();
    let earnings: i32 = bookings
        .iter()
        .filter(|b| b.status != "cancelled")
        .map(|b| b.total_amount)
        .sum();
    HttpResponse::Ok().json(serde_json::json!({"profile": profile, "availability": availability, "bookings": bookings, "stats": {"total_bookings": bookings.len(), "pending_bookings": pending, "earnings": earnings}}))
}

/// Public endpoint used by the frontend's Find tutors page.
#[get("/api/tutors")]
pub async fn list_tutors(pool: web::Data<PgPool>) -> impl Responder {
    let result = sqlx::query_as::<_, TutorListing>(
        r#"SELECT u.id, u.first_name, u.last_name, tp.county, tp.estate, tp.education_level, tp.subjects, tp.grades,
           tp.years_experience, tp.hourly_rate, tp.bio, tp.photo_url, tp.intro_video_url, tp.is_verified, tp.can_travel, tp.travel_radius_km
           FROM teacher_profiles tp JOIN users u ON u.id = tp.user_id
           WHERE u.role = 'teacher' AND u.is_active = TRUE
           -- Real tutor accounts lead the catalogue; seeded demo accounts
           -- remain available below them for local development.
           ORDER BY (u.email NOT LIKE '%@soma.test') DESC,
                    u.created_at DESC,
                    tp.is_verified DESC"#,
    ).fetch_all(pool.get_ref()).await;
    match result {
        Ok(tutors) => HttpResponse::Ok().json(tutors),
        Err(error) => {
            eprintln!("Tutor catalogue error: {error:?}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"message": "Could not load tutors"}))
        }
    }
}

/// Public tutor profile; only marketplace-safe profile fields are returned.
#[get("/api/tutors/{id}")]
pub async fn get_tutor_profile(pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let result = sqlx::query_as::<_, TutorListing>(
        "SELECT u.id,u.first_name,u.last_name,tp.county,tp.estate,tp.education_level,tp.subjects,tp.grades,tp.years_experience,tp.hourly_rate,tp.bio,tp.photo_url,tp.intro_video_url,tp.is_verified,tp.can_travel,tp.travel_radius_km FROM teacher_profiles tp JOIN users u ON u.id=tp.user_id WHERE tp.user_id=$1 AND u.role='teacher' AND u.is_active=TRUE"
    ).bind(path.into_inner()).fetch_optional(pool.get_ref()).await;
    match result {
        Ok(Some(profile)) => HttpResponse::Ok().json(profile),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({"message":"Tutor not found"})),
        Err(error) => {
            eprintln!("Tutor profile error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/api/tutors/{id}/schedule")]
pub async fn public_schedule(pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let tutor_id = path.into_inner();
    let travel = sqlx::query_as::<_, (bool, Option<i32>)>(
        "SELECT can_travel, travel_radius_km FROM teacher_profiles WHERE user_id=$1",
    )
    .bind(tutor_id)
    .fetch_optional(pool.get_ref())
    .await;
    let (can_travel, travel_radius_km) = match travel {
        Ok(Some(value)) => value,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({"message":"Tutor not found"}));
        }
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let availability = sqlx::query_as::<_, AvailabilityWindow>("SELECT weekday, start_time::text AS start_time, end_time::text AS end_time FROM teacher_availability WHERE teacher_id=$1 ORDER BY weekday")
        .bind(tutor_id).fetch_all(pool.get_ref()).await;
    let availability = match availability {
        Ok(value) => value,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let until = Utc::now().date_naive() + Duration::days(14);
    let booked = sqlx::query_as::<_, BookedSlot>("SELECT scheduled_date, scheduled_time, duration_minutes, status FROM tutor_bookings WHERE tutor_id=$1 AND scheduled_date >= CURRENT_DATE AND scheduled_date <= $2 AND status IN ('pending','confirmed')")
        .bind(tutor_id).bind(until).fetch_all(pool.get_ref()).await;
    match booked {
        Ok(booked) => HttpResponse::Ok().json(serde_json::json!({"can_travel":can_travel,"travel_radius_km":travel_radius_km,"availability":availability,"booked_slots":booked})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
