use crate::middleware::auth::AuthUser;
use crate::models::admin::{
    ActiveUpdate, AdminUser, AuditLog, CentreSubmission, DashboardMetrics, PlatformSettings,
    RoleUpdate, TutorVerification, TutorVerificationUpdate,
};
use actix_web::{HttpResponse, Responder, delete, get, put, web};
use sqlx::PgPool;
use uuid::Uuid;

fn admin_id(user: &AuthUser) -> Result<Uuid, HttpResponse> {
    if !matches!(user.role.as_str(), "admin" | "super_admin") {
        return Err(
            HttpResponse::Forbidden().json(serde_json::json!({"message": "Admin access required"}))
        );
    }
    Uuid::parse_str(&user.user_id).map_err(|_| HttpResponse::Unauthorized().finish())
}

#[get("/api/admin/dashboard")]
pub async fn dashboard_metrics(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    if user.role != "super_admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"message": "Super admin access required"}));
    }
    let query = r#"
        SELECT
            (SELECT COUNT(*) FROM users) AS total_users,
            (SELECT COUNT(*) FROM users WHERE created_at >= NOW() - INTERVAL '30 days') AS new_users_30d,
            (SELECT COUNT(*) FROM users WHERE role = 'teacher' AND is_active) AS active_tutors,
            (SELECT COUNT(*) FROM tutor_bookings WHERE created_at >= NOW() - INTERVAL '30 days' AND status != 'cancelled') AS bookings_30d,
            (SELECT COALESCE(SUM(total_amount), 0) FROM tutor_bookings WHERE created_at >= NOW() - INTERVAL '30 days' AND status != 'cancelled') AS gmv_30d
    "#;
    match sqlx::query_as::<_, DashboardMetrics>(query)
        .fetch_one(pool.get_ref())
        .await
    {
        Ok(metrics) => HttpResponse::Ok().json(metrics),
        Err(error) => {
            eprintln!("Dashboard metrics error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/api/admin/users")]
pub async fn list_users(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    if let Err(response) = admin_id(&user) {
        return response;
    }
    match sqlx::query_as::<_, AdminUser>("SELECT id, email, first_name, last_name, role::text AS role, is_active, created_at FROM users ORDER BY created_at DESC").fetch_all(pool.get_ref()).await {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(error) => { eprintln!("Admin users error: {error:?}"); HttpResponse::InternalServerError().finish() }
    }
}

#[put("/api/admin/users/{id}/role")]
pub async fn update_role(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<RoleUpdate>,
) -> impl Responder {
    let actor_id = match admin_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    if user.role != "super_admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"message": "Only a super admin can change access"}));
    }
    if !matches!(
        body.role.as_str(),
        "student" | "parent" | "teacher" | "admin"
    ) {
        return HttpResponse::BadRequest().json(serde_json::json!({"message": "Invalid role"}));
    }
    let target_id = path.into_inner();
    let result =
        sqlx::query("UPDATE users SET role = $1::user_role, updated_at = NOW() WHERE id = $2")
            .bind(&body.role)
            .bind(target_id)
            .execute(pool.get_ref())
            .await;
    match result {
        Ok(result) if result.rows_affected() == 1 => {
            let _ = sqlx::query("INSERT INTO audit_logs (actor_id, action, target_id, metadata) VALUES ($1, $2, $3, $4)")
                .bind(actor_id).bind("role_updated").bind(target_id).bind(serde_json::json!({"role": body.role})).execute(pool.get_ref()).await;
            HttpResponse::Ok().json(serde_json::json!({"message": "User access updated"}))
        }
        Ok(_) => HttpResponse::NotFound().json(serde_json::json!({"message": "User not found"})),
        Err(error) => {
            eprintln!("Role update error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[put("/api/admin/users/{id}/active")]
pub async fn update_active(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<ActiveUpdate>,
) -> impl Responder {
    let actor_id = match admin_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    if user.role != "super_admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"message":"Only a super admin can suspend accounts"}));
    }
    let target_id = path.into_inner();
    if target_id == actor_id && !body.is_active {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message":"You cannot suspend your own account"}));
    }
    match sqlx::query("UPDATE users SET is_active=$1, updated_at=NOW() WHERE id=$2")
        .bind(body.is_active)
        .bind(target_id)
        .execute(pool.get_ref())
        .await
    {
        Ok(result) if result.rows_affected() == 1 => {
            let _ = sqlx::query(
                "INSERT INTO audit_logs (actor_id,action,target_id,metadata) VALUES ($1,$2,$3,$4)",
            )
            .bind(actor_id)
            .bind("account_status_updated")
            .bind(target_id)
            .bind(serde_json::json!({"is_active":body.is_active}))
            .execute(pool.get_ref())
            .await;
            HttpResponse::Ok().json(serde_json::json!({"message":"Account status updated"}))
        }
        Ok(_) => HttpResponse::NotFound().json(serde_json::json!({"message":"User not found"})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/api/admin/tutor-verifications")]
pub async fn list_tutor_verifications(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    if let Err(response) = admin_id(&user) {
        return response;
    }
    let query = r#"SELECT u.id AS user_id, u.first_name, u.last_name, u.email, tp.county, tp.estate, tp.subjects, tp.photo_url, tp.intro_video_url, tp.updated_at AS created_at
        FROM teacher_profiles tp JOIN users u ON u.id = tp.user_id
        WHERE u.is_active = TRUE AND tp.is_verified = FALSE AND (tp.photo_url IS NOT NULL OR tp.intro_video_url IS NOT NULL)
        ORDER BY tp.updated_at ASC"#;
    match sqlx::query_as::<_, TutorVerification>(query)
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(profiles) => HttpResponse::Ok().json(profiles),
        Err(error) => {
            eprintln!("Tutor verification listing error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[put("/api/admin/tutor-verifications/{id}")]
pub async fn update_tutor_verification(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<TutorVerificationUpdate>,
) -> impl Responder {
    let actor_id = match admin_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    if user.role != "super_admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"message":"Only a super admin can verify tutors"}));
    }
    let tutor_id = path.into_inner();
    match sqlx::query(
        "UPDATE teacher_profiles SET is_verified=$1, updated_at=NOW() WHERE user_id=$2",
    )
    .bind(body.is_verified)
    .bind(tutor_id)
    .execute(pool.get_ref())
    .await
    {
        Ok(result) if result.rows_affected() == 1 => {
            let _ = sqlx::query("INSERT INTO audit_logs (actor_id, action, target_id, metadata) VALUES ($1, $2, $3, $4)")
                .bind(actor_id).bind("tutor_verification_updated").bind(tutor_id).bind(serde_json::json!({"is_verified": body.is_verified})).execute(pool.get_ref()).await;
            HttpResponse::Ok().json(serde_json::json!({"message": "Tutor verification updated"}))
        }
        Ok(_) => {
            HttpResponse::NotFound().json(serde_json::json!({"message": "Tutor profile not found"}))
        }
        Err(error) => {
            eprintln!("Tutor verification update error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Permanently removes an account and its user-owned records. This is limited
/// to super admins and protects the current and final super-admin accounts.
#[delete("/api/admin/users/{id}")]
pub async fn delete_user(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let actor_id = match admin_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    if user.role != "super_admin" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"message":"Only a super admin can delete accounts"}));
    }
    let target_id = path.into_inner();
    if target_id == actor_id {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message":"You cannot delete your own account"}));
    }
    let target_role =
        match sqlx::query_scalar::<_, String>("SELECT role::text FROM users WHERE id=$1")
            .bind(target_id)
            .fetch_optional(pool.get_ref())
            .await
        {
            Ok(Some(role)) => role,
            Ok(None) => {
                return HttpResponse::NotFound()
                    .json(serde_json::json!({"message":"User not found"}));
            }
            Err(error) => {
                eprintln!("User delete lookup error: {error:?}");
                return HttpResponse::InternalServerError().finish();
            }
        };
    if target_role == "super_admin" {
        match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE role='super_admin'")
            .fetch_one(pool.get_ref())
            .await
        {
            Ok(1) => {
                return HttpResponse::BadRequest().json(
                    serde_json::json!({"message":"You cannot delete the final super admin"}),
                );
            }
            Err(error) => {
                eprintln!("Super admin count error: {error:?}");
                return HttpResponse::InternalServerError().finish();
            }
            _ => {}
        }
    }
    match sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(target_id)
        .execute(pool.get_ref())
        .await
    {
        Ok(result) if result.rows_affected() == 1 => {
            let _ = sqlx::query("INSERT INTO audit_logs (actor_id,action,target_id,metadata) VALUES ($1,'user_deleted',$2,$3)")
                .bind(actor_id).bind(target_id).bind(serde_json::json!({"role": target_role})).execute(pool.get_ref()).await;
            HttpResponse::Ok().json(serde_json::json!({"message":"User deleted"}))
        }
        Ok(_) => HttpResponse::NotFound().json(serde_json::json!({"message":"User not found"})),
        Err(error) => {
            eprintln!("User delete error: {error:?}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"message":"Could not delete user"}))
        }
    }
}

#[get("/api/admin/centre-submissions")]
pub async fn list_centre_submissions(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    if let Err(response) = admin_id(&user) {
        return response;
    }
    let query = r#"SELECT c.id, c.name, c.county, c.estate, c.subjects, c.monthly_fee, CONCAT(u.first_name, ' ', u.last_name) AS owner_name, u.email AS owner_email, c.created_at
        FROM tuition_centres c JOIN users u ON u.id = c.owner_id WHERE c.is_active = FALSE ORDER BY c.created_at ASC"#;
    match sqlx::query_as::<_, CentreSubmission>(query)
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(submissions) => HttpResponse::Ok().json(submissions),
        Err(error) => {
            eprintln!("Centre submission listing error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[put("/api/admin/centre-submissions/{id}/approve")]
pub async fn approve_centre_submission(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let actor_id = match admin_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let centre_id = path.into_inner();
    match sqlx::query("UPDATE tuition_centres SET is_active=TRUE, updated_at=NOW() WHERE id=$1 AND is_active=FALSE").bind(centre_id).execute(pool.get_ref()).await {
        Ok(result) if result.rows_affected() == 1 => { let _ = sqlx::query("INSERT INTO audit_logs (actor_id, action, target_id) VALUES ($1, 'centre_submission_approved', $2)").bind(actor_id).bind(centre_id).execute(pool.get_ref()).await; HttpResponse::Ok().json(serde_json::json!({"message":"Centre listing approved"})) }
        Ok(_) => HttpResponse::NotFound().json(serde_json::json!({"message":"Pending centre submission not found"})),
        Err(error) => { eprintln!("Centre approval error: {error:?}"); HttpResponse::InternalServerError().finish() }
    }
}

#[get("/api/admin/settings")]
pub async fn get_settings(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    if user.role != "super_admin" {
        return HttpResponse::Forbidden().finish();
    }
    match sqlx::query_as::<_, PlatformSettings>("SELECT maintenance,signups,payments,ai_recommendations,public_teacher_directory FROM platform_settings WHERE id=TRUE").fetch_one(pool.get_ref()).await { Ok(value) => HttpResponse::Ok().json(value), Err(_) => HttpResponse::InternalServerError().finish() }
}

#[put("/api/admin/settings")]
pub async fn update_settings(
    user: AuthUser,
    pool: web::Data<PgPool>,
    body: web::Json<PlatformSettings>,
) -> impl Responder {
    let actor_id = match admin_id(&user) {
        Ok(id) => id,
        Err(response) => return response,
    };
    if user.role != "super_admin" {
        return HttpResponse::Forbidden().finish();
    }
    match sqlx::query("UPDATE platform_settings SET maintenance=$1,signups=$2,payments=$3,ai_recommendations=$4,public_teacher_directory=$5,updated_at=NOW() WHERE id=TRUE").bind(body.maintenance).bind(body.signups).bind(body.payments).bind(body.ai_recommendations).bind(body.public_teacher_directory).execute(pool.get_ref()).await {
        Ok(_) => { let _ = sqlx::query("INSERT INTO audit_logs (actor_id,action,metadata) VALUES ($1,'platform_settings_updated',$2)").bind(actor_id).bind(serde_json::json!({"maintenance":body.maintenance,"signups":body.signups,"payments":body.payments,"ai_recommendations":body.ai_recommendations,"public_teacher_directory":body.public_teacher_directory})).execute(pool.get_ref()).await; HttpResponse::Ok().json(serde_json::json!({"message":"Platform settings saved"})) }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/api/admin/logs")]
pub async fn list_logs(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    if let Err(response) = admin_id(&user) {
        return response;
    }
    match sqlx::query_as::<_, AuditLog>("SELECT id, actor_id, action, target_id, metadata, created_at FROM audit_logs ORDER BY created_at DESC LIMIT 200").fetch_all(pool.get_ref()).await {
        Ok(logs) => HttpResponse::Ok().json(logs),
        Err(error) => { eprintln!("Audit logs error: {error:?}"); HttpResponse::InternalServerError().finish() }
    }
}
