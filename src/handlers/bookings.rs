use actix_web::{HttpResponse, Responder, get, post, put, web};
use sqlx::PgPool;
use uuid::Uuid;
use std::env;

use crate::middleware::auth::AuthUser;
use crate::middleware::roles::Student;
use crate::models::bookings::{BookingDetail, BookingResponse, CreateBooking};

fn user_id(user: &AuthUser) -> Result<Uuid, HttpResponse> {
    Uuid::parse_str(&user.user_id).map_err(|_| {
        HttpResponse::Unauthorized().json(serde_json::json!({"message":"Invalid user ID"}))
    })
}

fn detail_query() -> &'static str {
    r#"SELECT b.id, b.tutor_id AS teacher_id, b.student_id,
        CONCAT(t.first_name, ' ', t.last_name) AS teacher_name,
        CONCAT(s.first_name, ' ', s.last_name) AS student_name,
        b.starts_at, b.ends_at, b.session_type, b.subject, b.grade_level,
        b.total_amount, b.status, b.notes, b.meeting_url
       FROM tutor_bookings b
       JOIN users t ON t.id=b.tutor_id JOIN users s ON s.id=b.student_id"#
}

#[post("/api/student/bookings")]
pub async fn create_booking(
    student: Student,
    pool: web::Data<PgPool>,
    data: web::Json<CreateBooking>,
) -> impl Responder {
    let student_id = match user_id(&student.0) {
        Ok(id) => id,
        Err(response) => return response,
    };
    if !matches!(data.session_type.as_str(), "online" | "physical")
        || data.subject.trim().is_empty()
    {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message":"Choose a subject and a valid session type"}));
    }
    if data.starts_at <= chrono::Utc::now() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message":"Lessons must be booked in the future"}));
    }
    // The INSERT validates availability and computes duration/price server-side.
    // PostgreSQL's exclusion constraint is the final concurrency-safe guard.
    let result = sqlx::query_as::<_, BookingResponse>(r#"
        INSERT INTO tutor_bookings
          (student_id,tutor_id,availability_id,starts_at,ends_at,session_type,subject,grade_level,notes,scheduled_date,scheduled_time,mode,duration_minutes,total_amount)
        SELECT $1,a.teacher_id,a.id,$3,$3 + make_interval(mins => a.slot_duration_minutes),$4,$5,$6,$7,
          ($3 AT TIME ZONE 'Africa/Nairobi')::date,to_char($3 AT TIME ZONE 'Africa/Nairobi','HH12:MI AM'),
          CASE WHEN $4='physical' THEN 'home' ELSE 'online' END,a.slot_duration_minutes,
          CEIL(tp.hourly_rate * a.slot_duration_minutes / 60.0)::integer
        FROM teacher_availability a
        JOIN teacher_profiles tp ON tp.user_id=a.teacher_id
        JOIN users teacher ON teacher.id=a.teacher_id AND teacher.is_active=TRUE
        WHERE a.id=$2 AND a.teacher_id=$8 AND $3 > NOW()
          AND (a.session_type='both' OR a.session_type=$4)
          AND (a.subject IS NULL OR a.subject=$5)
          AND (a.grade_level IS NULL OR a.grade_level=$6)
          AND $5=ANY(tp.subjects) AND ($6 IS NULL OR $6=ANY(tp.grades))
          AND a.available_from <= ($3 AT TIME ZONE 'Africa/Nairobi')::date
          AND (a.available_until IS NULL OR a.available_until >= ($3 AT TIME ZONE 'Africa/Nairobi')::date)
          AND ((a.repeats_weekly AND a.weekday=EXTRACT(DOW FROM ($3 AT TIME ZONE 'Africa/Nairobi')::date))
               OR (NOT a.repeats_weekly AND a.available_from=($3 AT TIME ZONE 'Africa/Nairobi')::date))
          AND ($3 AT TIME ZONE 'Africa/Nairobi')::time >= a.start_time
          AND ($3 AT TIME ZONE 'Africa/Nairobi')::time + make_interval(mins => a.slot_duration_minutes) <= a.end_time
          AND MOD(EXTRACT(EPOCH FROM (($3 AT TIME ZONE 'Africa/Nairobi')::time-a.start_time))::integer,a.slot_duration_minutes*60)=0
        RETURNING id,tutor_id AS teacher_id,student_id,starts_at,ends_at,status
    "#).bind(student_id).bind(data.availability_id).bind(data.starts_at).bind(&data.session_type)
      .bind(data.subject.trim()).bind(&data.grade_level).bind(&data.notes).bind(data.tutor_id)
      .fetch_optional(pool.get_ref()).await;
    match result {
        Ok(Some(booking)) => HttpResponse::Created().json(serde_json::json!({"message":"Lesson booked successfully","booking":booking})),
        Ok(None) => HttpResponse::Conflict().json(serde_json::json!({"message":"That time is unavailable or no longer matches this teacher's schedule"})),
        Err(sqlx::Error::Database(error)) if error.code().as_deref()==Some("23P01") => HttpResponse::Conflict().json(serde_json::json!({"message":"That tutor time has just been booked. Choose another slot."})),
        Err(error) => { eprintln!("Booking error: {error:?}"); HttpResponse::InternalServerError().json(serde_json::json!({"message":"Could not create booking"})) }
    }
}

#[get("/bookings")]
pub async fn student_bookings(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    if user.role != "student" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"message":"Student access required"}));
    }
    let id = match user_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let sql = format!(
        "{} WHERE b.student_id=$1 ORDER BY b.starts_at DESC",
        detail_query()
    );
    match sqlx::query_as::<_, BookingDetail>(&sql)
        .bind(id)
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(error) => {
            eprintln!("Student bookings error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/teacher/bookings")]
pub async fn teacher_bookings(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    if user.role != "teacher" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"message":"Teacher access required"}));
    }
    let id = match user_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let sql = format!(
        "{} WHERE b.tutor_id=$1 ORDER BY b.starts_at DESC",
        detail_query()
    );
    match sqlx::query_as::<_, BookingDetail>(&sql)
        .bind(id)
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(rows) => HttpResponse::Ok().json(rows),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/bookings/{id}")]
pub async fn booking_detail(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let id = match user_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let sql = format!(
        "{} WHERE b.id=$1 AND ($2 IN (b.student_id,b.tutor_id) OR $3 IN ('admin','super_admin'))",
        detail_query()
    );
    match sqlx::query_as::<_, BookingDetail>(&sql)
        .bind(path.into_inner())
        .bind(id)
        .bind(&user.role)
        .fetch_optional(pool.get_ref())
        .await
    {
        Ok(Some(row)) => HttpResponse::Ok().json(row),
        Ok(None) => {
            HttpResponse::NotFound().json(serde_json::json!({"message":"Booking not found"}))
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[put("/bookings/{id}/cancel")]
pub async fn cancel_booking(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let id = match user_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let result = sqlx::query("UPDATE tutor_bookings SET status='cancelled',updated_at=NOW() WHERE id=$1 AND (student_id=$2 OR tutor_id=$2 OR $3 IN ('admin','super_admin')) AND status IN ('pending','confirmed') AND starts_at>NOW()").bind(path.into_inner()).bind(id).bind(&user.role).execute(pool.get_ref()).await;
    match result {
        Ok(value) if value.rows_affected() == 1 => {
            HttpResponse::Ok().json(serde_json::json!({"message":"Booking cancelled"}))
        }
        Ok(_) => HttpResponse::Conflict()
            .json(serde_json::json!({"message":"This booking cannot be cancelled"})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[put("/bookings/{id}/confirm")]
pub async fn confirm_booking(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    update_status(user, pool, path.into_inner(), "confirmed").await
}
#[put("/bookings/{id}/complete")]
pub async fn complete_booking(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    update_status(user, pool, path.into_inner(), "completed").await
}

async fn update_status(
    user: AuthUser,
    pool: web::Data<PgPool>,
    booking_id: Uuid,
    status: &str,
) -> HttpResponse {
    let id = match user_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    if user.role != "teacher" && !matches!(user.role.as_str(), "admin" | "super_admin") {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"message":"Teacher access required"}));
    }
    let result = sqlx::query("UPDATE tutor_bookings SET status=$1,updated_at=NOW() WHERE id=$2 AND (tutor_id=$3 OR $4 IN ('admin','super_admin')) AND status='pending'").bind(status).bind(booking_id).bind(id).bind(&user.role).execute(pool.get_ref()).await;
    match result {
        Ok(value) if value.rows_affected() == 1 => {
            // Online confirmed bookings own exactly one classroom. The database
            // unique booking_id constraint keeps this idempotent under retries.
            if status == "confirmed" {
                let provider = env::var("CLASSROOM_PROVIDER").unwrap_or_else(|_| "internal".to_string());
                let room_id = if provider == "zoom" {
                    env::var("ZOOM_MEETING_URL").unwrap_or_else(|_| format!("soma-{}", booking_id))
                } else { format!("soma-{}", booking_id) };
                let create = sqlx::query(r#"
                    INSERT INTO classroom_sessions (booking_id,title,room_id,provider)
                    SELECT b.id, CONCAT(b.subject, COALESCE(CONCAT(' — ', b.grade_level), '')), $2, $3
                    FROM tutor_bookings b WHERE b.id=$1 AND b.session_type='online'
                    ON CONFLICT (booking_id) DO NOTHING
                "#).bind(booking_id).bind(room_id).bind(provider).execute(pool.get_ref()).await;
                if let Err(error) = create {
                    eprintln!("Classroom creation error: {error:?}");
                    return HttpResponse::InternalServerError().json(serde_json::json!({"message":"Booking was confirmed but the classroom could not be created"}));
                }
            }
            HttpResponse::Ok()
                .json(serde_json::json!({"message":"Booking updated","status":status}))
        }
        Ok(_) => HttpResponse::Conflict()
            .json(serde_json::json!({"message":"Booking is no longer pending or is not yours"})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
