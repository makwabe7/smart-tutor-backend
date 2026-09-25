use actix_web::{HttpResponse, Responder, get, put, web};
use sqlx::PgPool;
use uuid::Uuid;

use crate::middleware::auth::AuthUser;
use crate::models::notifications::Notification;

fn user_id(user: &AuthUser) -> Result<Uuid, HttpResponse> {
    Uuid::parse_str(&user.user_id).map_err(|_| {
        HttpResponse::Unauthorized().json(serde_json::json!({"message": "Invalid user ID"}))
    })
}

#[get("")]
pub async fn list(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    let user_id = match user_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    match sqlx::query_as::<_, Notification>(
        "SELECT id, title, body, category, href, is_read, created_at FROM user_notifications WHERE user_id = $1 ORDER BY created_at DESC LIMIT 100",
    ).bind(user_id).fetch_all(pool.get_ref()).await {
        Ok(notifications) => HttpResponse::Ok().json(notifications),
        Err(error) => { eprintln!("Notifications list error: {error:?}"); HttpResponse::InternalServerError().json(serde_json::json!({"message": "Could not load notifications"})) }
    }
}

#[put("/{id}/read")]
pub async fn mark_read(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let user_id = match user_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    match sqlx::query("UPDATE user_notifications SET is_read = TRUE WHERE id = $1 AND user_id = $2")
        .bind(path.into_inner())
        .bind(user_id)
        .execute(pool.get_ref())
        .await
    {
        Ok(result) if result.rows_affected() == 1 => HttpResponse::NoContent().finish(),
        Ok(_) => {
            HttpResponse::NotFound().json(serde_json::json!({"message": "Notification not found"}))
        }
        Err(error) => {
            eprintln!("Notification read error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[put("/read-all")]
pub async fn mark_all_read(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    let user_id = match user_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    match sqlx::query(
        "UPDATE user_notifications SET is_read = TRUE WHERE user_id = $1 AND is_read = FALSE",
    )
    .bind(user_id)
    .execute(pool.get_ref())
    .await
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(error) => {
            eprintln!("Notifications read-all error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
