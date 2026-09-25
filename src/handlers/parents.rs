use actix_web::{HttpResponse, Responder, get, post, put, web};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::middleware::auth::AuthUser;
use crate::models::parents::{AddManagedChild, LinkRequest, ParentStudent, RequestStudentLink};

fn account_id(user: &AuthUser, required_role: &str) -> Result<Uuid, HttpResponse> {
    if user.role != required_role {
        return Err(HttpResponse::Forbidden()
            .json(serde_json::json!({"message": format!("{} access required", required_role)})));
    }
    Uuid::parse_str(&user.user_id).map_err(|_| {
        HttpResponse::Unauthorized().json(serde_json::json!({"message": "Invalid user ID"}))
    })
}

#[post("/children")]
pub async fn add_managed_child(
    user: AuthUser,
    pool: web::Data<PgPool>,
    body: web::Json<AddManagedChild>,
) -> impl Responder {
    let parent_id = match account_id(&user, "parent") {
        Ok(id) => id,
        Err(response) => return response,
    };
    let name = body.child_name.trim();
    if name.is_empty() || name.len() > 200 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message": "Enter the child's name"}));
    }
    let relationship = body.relationship.as_deref().unwrap_or("Parent").trim();
    let result = sqlx::query_as::<_, ParentStudent>(
        "INSERT INTO parent_students (parent_id, child_name, grade_level, relationship, status) VALUES ($1,$2,$3,$4,'managed') RETURNING id, child_name, grade_level, relationship, status, student_id, 0::BIGINT AS booking_count, NULL::TIMESTAMPTZ AS last_booking_at",
    ).bind(parent_id).bind(name).bind(&body.grade_level).bind(relationship).fetch_one(pool.get_ref()).await;
    match result {
        Ok(child) => HttpResponse::Created().json(child),
        Err(error) => {
            eprintln!("Managed child error: {error:?}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"message": "Could not add child"}))
        }
    }
}

#[post("/student-links")]
pub async fn request_student_link(
    user: AuthUser,
    pool: web::Data<PgPool>,
    body: web::Json<RequestStudentLink>,
) -> impl Responder {
    let parent_id = match account_id(&user, "parent") {
        Ok(id) => id,
        Err(response) => return response,
    };
    let student = sqlx::query_as::<_, (Uuid, String, String)>("SELECT id, first_name, last_name FROM users WHERE LOWER(email) = LOWER($1) AND role = 'student' AND is_active = TRUE")
        .bind(body.student_email.trim()).fetch_optional(pool.get_ref()).await;
    let (student_id, first_name, last_name) = match student {
        Ok(Some(student)) => student,
        Ok(None) => {
            return HttpResponse::NotFound().json(
                serde_json::json!({"message": "No active student account matches that email"}),
            );
        }
        Err(error) => {
            eprintln!("Student lookup error: {error:?}");
            return HttpResponse::InternalServerError().finish();
        }
    };
    if student_id == parent_id {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message": "You cannot link your own account"}));
    }
    let relationship = body.relationship.as_deref().unwrap_or("Parent").trim();
    let child_name = format!("{} {}", first_name, last_name);
    let link = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO parent_students (parent_id, student_id, child_name, relationship, status) VALUES ($1,$2,$3,$4,'pending') ON CONFLICT (parent_id, student_id) WHERE student_id IS NOT NULL DO UPDATE SET relationship = EXCLUDED.relationship, status = 'pending', updated_at = NOW() RETURNING id",
    ).bind(parent_id).bind(student_id).bind(&child_name).bind(relationship).fetch_one(pool.get_ref()).await;
    match link {
        Ok(_) => {
            let parent_name = sqlx::query_scalar::<_, String>(
                "SELECT CONCAT(first_name, ' ', last_name) FROM users WHERE id = $1",
            )
            .bind(parent_id)
            .fetch_optional(pool.get_ref())
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "A parent".to_string());
            let _ = sqlx::query("INSERT INTO user_notifications (user_id, title, body, category, href) VALUES ($1,$2,$3,'system',$4)")
                .bind(student_id).bind("Parent connection request").bind(format!("{} wants to monitor your learning activity. Review and approve the request.", parent_name)).bind("/student/parent-links").execute(pool.get_ref()).await;
            HttpResponse::Created().json(serde_json::json!({"message": "Request sent. The student must approve it before you can monitor their account."}))
        }
        Err(error) => {
            eprintln!("Student link request error: {error:?}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"message": "Could not send connection request"}))
        }
    }
}

#[get("/children")]
pub async fn list_children(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    let parent_id = match account_id(&user, "parent") {
        Ok(id) => id,
        Err(response) => return response,
    };
    match sqlx::query_as::<_, ParentStudent>(
        "SELECT ps.id, ps.child_name, ps.grade_level, ps.relationship, ps.status, ps.student_id, COUNT(b.id)::BIGINT AS booking_count, MAX(b.created_at) AS last_booking_at FROM parent_students ps LEFT JOIN tutor_bookings b ON b.student_id = ps.student_id WHERE ps.parent_id = $1 GROUP BY ps.id ORDER BY ps.created_at DESC",
    ).bind(parent_id).fetch_all(pool.get_ref()).await {
        Ok(children) => HttpResponse::Ok().json(children),
        Err(error) => { eprintln!("Parent children error: {error:?}"); HttpResponse::InternalServerError().finish() }
    }
}

#[get("/requests")]
pub async fn list_link_requests(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    let student_id = match account_id(&user, "student") {
        Ok(id) => id,
        Err(response) => return response,
    };
    match sqlx::query_as::<_, LinkRequest>(
        "SELECT ps.id, CONCAT(u.first_name, ' ', u.last_name) AS parent_name, ps.relationship, ps.created_at FROM parent_students ps JOIN users u ON u.id = ps.parent_id WHERE ps.student_id = $1 AND ps.status = 'pending' ORDER BY ps.created_at DESC",
    ).bind(student_id).fetch_all(pool.get_ref()).await {
        Ok(requests) => HttpResponse::Ok().json(requests),
        Err(error) => { eprintln!("Link requests error: {error:?}"); HttpResponse::InternalServerError().finish() }
    }
}

#[put("/requests/{id}/accept")]
pub async fn accept_link_request(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let student_id = match account_id(&user, "student") {
        Ok(id) => id,
        Err(response) => return response,
    };
    let result = sqlx::query("UPDATE parent_students SET status = 'linked', updated_at = NOW() WHERE id = $1 AND student_id = $2 AND status = 'pending' RETURNING parent_id")
        .bind(path.into_inner()).bind(student_id).fetch_optional(pool.get_ref()).await;
    match result {
        Ok(Some(row)) => {
            let parent_id: Uuid = row.get("parent_id");
            let _ = sqlx::query("INSERT INTO user_notifications (user_id, title, body, category, href) VALUES ($1,$2,$3,'system',$4)")
                .bind(parent_id).bind("Student connection approved").bind("You can now monitor this student's learning activity.").bind("/parent/progress").execute(pool.get_ref()).await;
            HttpResponse::NoContent().finish()
        }
        Ok(None) => HttpResponse::NotFound()
            .json(serde_json::json!({"message": "Connection request not found"})),
        Err(error) => {
            eprintln!("Accept link error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
