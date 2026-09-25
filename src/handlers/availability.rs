use crate::middleware::auth::AuthUser;
use crate::models::availability::{AvailabilityInput, AvailabilityRule, AvailableSlot};
use actix_web::{HttpResponse, Responder, delete, get, post, put, web};
use chrono::NaiveTime;
use sqlx::PgPool;
use uuid::Uuid;

fn teacher_id(user: &AuthUser) -> Result<Uuid, HttpResponse> {
    if user.role != "teacher" {
        return Err(HttpResponse::Forbidden()
            .json(serde_json::json!({"message":"Teacher access required"})));
    }
    Uuid::parse_str(&user.user_id).map_err(|_| HttpResponse::Unauthorized().finish())
}
fn parsed_times(input: &AvailabilityInput) -> Option<(NaiveTime, NaiveTime)> {
    let start = NaiveTime::parse_from_str(&input.start_time, "%H:%M").ok()?;
    let end = NaiveTime::parse_from_str(&input.end_time, "%H:%M").ok()?;
    (end > start).then_some((start, end))
}
fn valid(input: &AvailabilityInput) -> bool {
    parsed_times(input).is_some()
        && input.day_of_week >= 0
        && input.day_of_week <= 6
        && matches!(input.slot_duration_minutes, 30 | 45 | 60 | 90)
        && matches!(input.session_type.as_str(), "online" | "physical" | "both")
        && input
            .available_until
            .map(|date| date >= input.available_from)
            .unwrap_or(true)
}

#[post("/availability")]
pub async fn create_availability(
    user: AuthUser,
    pool: web::Data<PgPool>,
    body: web::Json<AvailabilityInput>,
) -> impl Responder {
    let teacher_id = match teacher_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    if !valid(&body) {
        return HttpResponse::BadRequest().json(serde_json::json!({"message":"Enter a valid availability range, duration, and session type"}));
    }
    let (start_time, end_time) = parsed_times(&body).expect("validated times");
    let sql = "INSERT INTO teacher_availability (teacher_id,weekday,start_time,end_time,slot_duration_minutes,session_type,subject,grade_level,available_from,available_until,repeats_weekly,updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,NOW()) RETURNING id,teacher_id,weekday AS day_of_week,start_time,end_time,slot_duration_minutes,session_type,subject,grade_level,available_from,available_until,repeats_weekly";
    match sqlx::query_as::<_, AvailabilityRule>(sql)
        .bind(teacher_id)
        .bind(body.day_of_week)
        .bind(start_time)
        .bind(end_time)
        .bind(body.slot_duration_minutes)
        .bind(&body.session_type)
        .bind(&body.subject)
        .bind(&body.grade_level)
        .bind(body.available_from)
        .bind(body.available_until)
        .bind(body.repeats_weekly)
        .fetch_one(pool.get_ref())
        .await
    {
        Ok(rule) => HttpResponse::Created()
            .json(serde_json::json!({"message":"Availability added","availability":rule})),
        Err(error) => {
            eprintln!("Availability create error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[put("/availability/{id}")]
pub async fn update_availability(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<AvailabilityInput>,
) -> impl Responder {
    let teacher_id = match teacher_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    if !valid(&body) {
        return HttpResponse::BadRequest().json(serde_json::json!({"message":"Enter a valid availability range, duration, and session type"}));
    }
    let (start_time, end_time) = parsed_times(&body).expect("validated times");
    let sql = "UPDATE teacher_availability SET weekday=$1,start_time=$2,end_time=$3,slot_duration_minutes=$4,session_type=$5,subject=$6,grade_level=$7,available_from=$8,available_until=$9,repeats_weekly=$10,updated_at=NOW() WHERE id=$11 AND teacher_id=$12 RETURNING id,teacher_id,weekday AS day_of_week,start_time,end_time,slot_duration_minutes,session_type,subject,grade_level,available_from,available_until,repeats_weekly";
    match sqlx::query_as::<_, AvailabilityRule>(sql)
        .bind(body.day_of_week)
        .bind(start_time)
        .bind(end_time)
        .bind(body.slot_duration_minutes)
        .bind(&body.session_type)
        .bind(&body.subject)
        .bind(&body.grade_level)
        .bind(body.available_from)
        .bind(body.available_until)
        .bind(body.repeats_weekly)
        .bind(path.into_inner())
        .bind(teacher_id)
        .fetch_optional(pool.get_ref())
        .await
    {
        Ok(Some(rule)) => HttpResponse::Ok()
            .json(serde_json::json!({"message":"Availability updated","availability":rule})),
        Ok(None) => {
            HttpResponse::NotFound().json(serde_json::json!({"message":"Availability not found"}))
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[delete("/availability/{id}")]
pub async fn delete_availability(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let teacher_id = match teacher_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    match sqlx::query("DELETE FROM teacher_availability WHERE id=$1 AND teacher_id=$2")
        .bind(path.into_inner())
        .bind(teacher_id)
        .execute(pool.get_ref())
        .await
    {
        Ok(result) if result.rows_affected() == 1 => {
            HttpResponse::Ok().json(serde_json::json!({"message":"Availability deleted"}))
        }
        Ok(_) => {
            HttpResponse::NotFound().json(serde_json::json!({"message":"Availability not found"}))
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/{teacher_id}/availability")]
pub async fn public_availability(pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let sql = "SELECT id,teacher_id,weekday AS day_of_week,start_time,end_time,slot_duration_minutes,session_type,subject,grade_level,available_from,available_until,repeats_weekly FROM teacher_availability WHERE teacher_id=$1 ORDER BY weekday,start_time";
    match sqlx::query_as::<_, AvailabilityRule>(sql)
        .bind(path.into_inner())
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(error) => {
            eprintln!("Availability list error: {error:?}");
            HttpResponse::InternalServerError().json(
                serde_json::json!({"message":"Could not load availability. Please try again."}),
            )
        }
    }
}

#[get("/api/teachers/{teacher_id}/available-slots")]
pub async fn available_slots(pool: web::Data<PgPool>, path: web::Path<Uuid>) -> impl Responder {
    let sql = r#"WITH dates AS (SELECT d::date AS day FROM generate_series(CURRENT_DATE, CURRENT_DATE + 13, INTERVAL '1 day') d)
        SELECT a.id AS availability_id,
          (slot_timestamp AT TIME ZONE 'Africa/Nairobi') AS starts_at,
          ((slot_timestamp + make_interval(mins => a.slot_duration_minutes)) AT TIME ZONE 'Africa/Nairobi') AS ends_at,
          a.session_type,a.subject,a.grade_level,a.slot_duration_minutes
        FROM teacher_availability a CROSS JOIN dates
        CROSS JOIN LATERAL generate_series(
          dates.day::timestamp + a.start_time,
          dates.day::timestamp + a.end_time - make_interval(mins => a.slot_duration_minutes),
          make_interval(mins => a.slot_duration_minutes)
        ) slot_timestamp
        WHERE a.teacher_id=$1 AND a.available_from <= dates.day AND (a.available_until IS NULL OR a.available_until >= dates.day)
          AND ((a.repeats_weekly AND a.weekday=EXTRACT(DOW FROM dates.day)) OR (NOT a.repeats_weekly AND a.available_from=dates.day))
          AND (slot_timestamp AT TIME ZONE 'Africa/Nairobi') > NOW()
          AND NOT EXISTS (SELECT 1 FROM tutor_bookings b WHERE b.tutor_id=a.teacher_id AND b.status IN ('pending','confirmed') AND tstzrange(b.starts_at,b.ends_at,'[)') && tstzrange((slot_timestamp AT TIME ZONE 'Africa/Nairobi'),((slot_timestamp + make_interval(mins => a.slot_duration_minutes)) AT TIME ZONE 'Africa/Nairobi'),'[)'))
        ORDER BY starts_at"#;
    match sqlx::query_as::<_, AvailableSlot>(sql)
        .bind(path.into_inner())
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(error) => {
            eprintln!("Available slots error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
