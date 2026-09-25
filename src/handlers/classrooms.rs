use crate::middleware::auth::AuthUser;
use crate::models::classrooms::*;
use actix::{Actor, ActorContext, StreamHandler};
use actix_web::{HttpResponse, Responder, delete, get, post, web};
use actix_web_actors::ws;
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::Serialize;
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

fn uid(user: &AuthUser) -> Result<Uuid, HttpResponse> {
    Uuid::parse_str(&user.user_id).map_err(|_| {
        HttpResponse::Unauthorized()
            .json(serde_json::json!({"message":"Invalid authentication identity"}))
    })
}
const CLASSROOM: &str = r#"SELECT c.id,c.booking_id,c.title,c.room_id,c.provider,c.status,c.started_at,c.ended_at,b.starts_at,b.ends_at,b.subject,b.grade_level,CONCAT(t.first_name,' ',t.last_name) tutor_name,CONCAT(s.first_name,' ',s.last_name) student_name FROM classroom_sessions c JOIN tutor_bookings b ON b.id=c.booking_id JOIN users t ON t.id=b.tutor_id JOIN users s ON s.id=b.student_id"#;

#[derive(Serialize)]
struct VideoGrant {
    room: String,
    #[serde(rename = "roomJoin")]
    room_join: bool,
    #[serde(rename = "canPublish")]
    can_publish: bool,
    #[serde(rename = "canSubscribe")]
    can_subscribe: bool,
}
#[derive(Serialize)]
struct LiveKitClaims {
    iss: String,
    sub: String,
    name: String,
    nbf: i64,
    exp: i64,
    video: VideoGrant,
    metadata: String,
}
fn media_token(
    user_id: Uuid,
    role: &str,
    room_id: &str,
    name: &str,
) -> Result<(String, String), HttpResponse> {
    let key = env::var("LIVEKIT_API_KEY").map_err(|_| {
        HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({"message":"LiveKit is not configured"}))
    })?;
    let secret = env::var("LIVEKIT_API_SECRET").map_err(|_| {
        HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({"message":"LiveKit is not configured"}))
    })?;
    let url = env::var("LIVEKIT_URL").map_err(|_| {
        HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({"message":"LiveKit is not configured"}))
    })?;
    let now = Utc::now();
    let claims = LiveKitClaims {
        iss: key,
        sub: user_id.to_string(),
        name: name.to_string(),
        nbf: now.timestamp(),
        exp: (now + Duration::minutes(10)).timestamp(),
        video: VideoGrant {
            room: room_id.to_string(),
            room_join: true,
            can_publish: true,
            can_subscribe: true,
        },
        metadata: format!("{{\"soma_role\":\"{}\"}}", role),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| HttpResponse::InternalServerError().finish())?;
    Ok((url, token))
}
struct ClassroomSocket;
impl Actor for ClassroomSocket {
    type Context = ws::WebsocketContext<Self>;
}
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ClassroomSocket {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(bytes)) => ctx.pong(&bytes),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => {}
        }
    }
}
#[get("/ws/classrooms/{id}")]
pub async fn classroom_socket(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: actix_web::HttpRequest,
    stream: web::Payload,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    authorize(pool.get_ref(), path.into_inner(), &user)
        .await
        .map_err(|_| actix_web::error::ErrorForbidden("Classroom access denied"))?;
    ws::start(ClassroomSocket, &req, stream)
}

async fn authorize(
    pool: &PgPool,
    classroom_id: Uuid,
    user: &AuthUser,
) -> Result<(Uuid, String), HttpResponse> {
    let id = uid(user)?;
    let row = sqlx::query_as::<_, (String, Uuid, Uuid)>("SELECT b.status,b.tutor_id,b.student_id FROM classroom_sessions c JOIN tutor_bookings b ON b.id=c.booking_id WHERE c.id=$1")
        .bind(classroom_id).fetch_optional(pool).await.map_err(|_| HttpResponse::InternalServerError().finish())?;
    match row {
        Some((_, tutor_id, student_id)) if id != tutor_id && id != student_id => Err(HttpResponse::Forbidden().json(serde_json::json!({"message":"You are not a participant in this classroom"}))),
        Some((status, tutor_id, _)) if status == "confirmed" || status == "completed" => Ok((id, if id == tutor_id { "tutor".into() } else { "student".into() })),
        Some(_) => Err(HttpResponse::Forbidden().json(serde_json::json!({"message":"This classroom is not available for an unconfirmed booking"}))),
        None => Err(HttpResponse::NotFound().json(serde_json::json!({"message":"Classroom not found"}))),
    }
}

#[get("/classrooms")]
pub async fn list_classrooms(user: AuthUser, pool: web::Data<PgPool>) -> impl Responder {
    let id = match uid(&user) {
        Ok(x) => x,
        Err(e) => return e,
    };
    let sql = format!(
        "{} WHERE $1 IN (b.tutor_id,b.student_id) AND b.status IN ('confirmed','completed') ORDER BY b.starts_at DESC",
        CLASSROOM
    );
    match sqlx::query_as::<_, Classroom>(&sql)
        .bind(id)
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(x) => HttpResponse::Ok().json(x),
        Err(e) => {
            eprintln!("classrooms: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
#[get("/classrooms/{id}")]
pub async fn get_classroom(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let cid = path.into_inner();
    if let Err(e) = authorize(pool.get_ref(), cid, &user).await {
        return e;
    };
    let sql = format!("{} WHERE c.id=$1", CLASSROOM);
    match sqlx::query_as::<_, Classroom>(&sql)
        .bind(cid)
        .fetch_optional(pool.get_ref())
        .await
    {
        Ok(Some(x)) => HttpResponse::Ok().json(x),
        Ok(None) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
#[get("/bookings/{id}/classroom")]
pub async fn get_booking_classroom(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let id = match uid(&user) {
        Ok(x) => x,
        Err(e) => return e,
    };
    let sql = format!(
        "{} WHERE c.booking_id=$1 AND $2 IN (b.tutor_id,b.student_id) AND b.status IN ('confirmed','completed')",
        CLASSROOM
    );
    match sqlx::query_as::<_, Classroom>(&sql)
        .bind(path.into_inner())
        .bind(id)
        .fetch_optional(pool.get_ref())
        .await
    {
        Ok(Some(x)) => HttpResponse::Ok().json(x),
        Ok(None) => HttpResponse::NotFound()
            .json(serde_json::json!({"message":"No available classroom for this booking"})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/classrooms/{id}/join")]
pub async fn join_classroom(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let cid = path.into_inner();
    let (id, role) = match authorize(pool.get_ref(), cid, &user).await {
        Ok(x) => x,
        Err(e) => return e,
    };
    let allowed=sqlx::query_scalar::<_,bool>("SELECT starts_at - INTERVAL '15 minutes' <= NOW() AND ends_at + INTERVAL '30 minutes' >= NOW() FROM classroom_sessions c JOIN tutor_bookings b ON b.id=c.booking_id WHERE c.id=$1").bind(cid).fetch_one(pool.get_ref()).await.unwrap_or(false);
    if !allowed {
        return HttpResponse::Conflict().json(serde_json::json!({"message":"Joining is available 15 minutes before the lesson until 30 minutes after it ends"}));
    };
    let q = "INSERT INTO classroom_participants(classroom_id,user_id,role,joined_at,left_at) VALUES($1,$2,$3,NOW(),NULL) ON CONFLICT(classroom_id,user_id) DO UPDATE SET joined_at=COALESCE(classroom_participants.joined_at,NOW()),left_at=NULL";
    if sqlx::query(q)
        .bind(cid)
        .bind(id)
        .bind(&role)
        .execute(pool.get_ref())
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    };
    let q = "INSERT INTO classroom_attendance(classroom_id,user_id,joined_at,left_at,duration_seconds) VALUES($1,$2,NOW(),NULL,0) ON CONFLICT(classroom_id,user_id) DO UPDATE SET joined_at=COALESCE(classroom_attendance.joined_at,NOW()),left_at=NULL";
    let _ = sqlx::query(q)
        .bind(cid)
        .bind(id)
        .execute(pool.get_ref())
        .await;
    let room = match sqlx::query_as::<_, (String, String, String)>(
        "SELECT room_id,title,provider FROM classroom_sessions WHERE id=$1",
    )
    .bind(cid)
    .fetch_one(pool.get_ref())
    .await
    {
        Ok(value) => value,
        Err(_) => return HttpResponse::NotFound().finish(),
    };
    if room.2 == "zoom" {
        return HttpResponse::Ok().json(ClassroomJoin {
            role,
            provider: "zoom".to_string(),
            livekit_url: None,
            media_token: None,
            zoom_url: Some(room.0),
            expires_in_seconds: 0,
        });
    }
    match media_token(id, &role, &room.0, &room.1) {
        Ok((livekit_url, media_token)) => HttpResponse::Ok().json(ClassroomJoin {
            role,
            provider: "internal".to_string(),
            livekit_url: Some(livekit_url),
            media_token: Some(media_token),
            zoom_url: None,
            expires_in_seconds: 600,
        }),
        Err(response) => response,
    }
}
#[post("/classrooms/{id}/leave")]
pub async fn leave_classroom(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let cid = path.into_inner();
    let (id, _) = match authorize(pool.get_ref(), cid, &user).await {
        Ok(x) => x,
        Err(e) => return e,
    };
    let q = "UPDATE classroom_attendance SET left_at=NOW(),duration_seconds=GREATEST(0,EXTRACT(EPOCH FROM NOW()-joined_at)::int) WHERE classroom_id=$1 AND user_id=$2 AND left_at IS NULL";
    let _ = sqlx::query(q)
        .bind(cid)
        .bind(id)
        .execute(pool.get_ref())
        .await;
    let _=sqlx::query("UPDATE classroom_participants SET left_at=NOW() WHERE classroom_id=$1 AND user_id=$2 AND left_at IS NULL").bind(cid).bind(id).execute(pool.get_ref()).await;
    HttpResponse::Ok().json(serde_json::json!({"message":"Left classroom"}))
}
async fn change_state(
    user: AuthUser,
    pool: web::Data<PgPool>,
    cid: Uuid,
    state: &str,
) -> HttpResponse {
    let (_, role) = match authorize(pool.get_ref(), cid, &user).await {
        Ok(x) => x,
        Err(e) => return e,
    };
    if role != "tutor" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"message":"Only the tutor can change classroom status"}));
    };
    let q = if state == "live" {
        "UPDATE classroom_sessions SET status='live',started_at=COALESCE(started_at,NOW()),updated_at=NOW() WHERE id=$1 AND status='scheduled'"
    } else {
        "UPDATE classroom_sessions SET status='completed',ended_at=COALESCE(ended_at,NOW()),updated_at=NOW() WHERE id=$1 AND status IN ('scheduled','live')"
    };
    match sqlx::query(q).bind(cid).execute(pool.get_ref()).await {
        Ok(x) if x.rows_affected() == 1 => {
            HttpResponse::Ok().json(serde_json::json!({"status":state}))
        }
        Ok(_) => HttpResponse::Conflict()
            .json(serde_json::json!({"message":"Classroom is already in that state"})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
#[post("/classrooms/{id}/start")]
pub async fn start_classroom(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    change_state(user, pool, path.into_inner(), "live").await
}
#[post("/classrooms/{id}/end")]
pub async fn end_classroom(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    change_state(user, pool, path.into_inner(), "completed").await
}
#[get("/classrooms/{id}/messages")]
pub async fn list_messages(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let cid = path.into_inner();
    if let Err(e) = authorize(pool.get_ref(), cid, &user).await {
        return e;
    };
    let q = "SELECT m.id,m.sender_id,CONCAT(u.first_name,' ',u.last_name) sender_name,m.message,m.created_at FROM classroom_messages m JOIN users u ON u.id=m.sender_id WHERE m.classroom_id=$1 ORDER BY m.created_at";
    match sqlx::query_as::<_, ClassroomMessage>(q)
        .bind(cid)
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(x) => HttpResponse::Ok().json(x),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
#[post("/classrooms/{id}/messages")]
pub async fn send_message(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    data: web::Json<SendMessage>,
) -> impl Responder {
    let cid = path.into_inner();
    let (id, _) = match authorize(pool.get_ref(), cid, &user).await {
        Ok(x) => x,
        Err(e) => return e,
    };
    let message = data.message.trim();
    if message.is_empty() || message.len() > 4000 {
        return HttpResponse::UnprocessableEntity()
            .json(serde_json::json!({"message":"Message must be between 1 and 4000 characters"}));
    };
    match sqlx::query(
        "INSERT INTO classroom_messages(classroom_id,sender_id,message) VALUES($1,$2,$3)",
    )
    .bind(cid)
    .bind(id)
    .bind(message)
    .execute(pool.get_ref())
    .await
    {
        Ok(_) => HttpResponse::Created().json(serde_json::json!({"message":"Message sent"})),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
#[get("/classrooms/{id}/resources")]
pub async fn list_resources(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let cid = path.into_inner();
    if let Err(e) = authorize(pool.get_ref(), cid, &user).await {
        return e;
    };
    match sqlx::query_as::<_,ClassroomResource>("SELECT id,uploaded_by,name,file_url,resource_type,created_at FROM classroom_resources WHERE classroom_id=$1 ORDER BY created_at DESC").bind(cid).fetch_all(pool.get_ref()).await{Ok(x)=>HttpResponse::Ok().json(x),Err(_)=>HttpResponse::InternalServerError().finish()}
}
#[post("/classrooms/{id}/resources")]
pub async fn create_resource(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    data: web::Json<CreateResource>,
) -> impl Responder {
    let cid = path.into_inner();
    let (id, role) = match authorize(pool.get_ref(), cid, &user).await {
        Ok(x) => x,
        Err(e) => return e,
    };
    if role != "tutor" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"message":"Only tutors can add lesson resources"}));
    };
    let d = data.into_inner();
    if d.name.trim().is_empty()
        || d.name.len() > 255
        || d.file_url.trim().is_empty()
        || !matches!(
            d.resource_type.as_str(),
            "pdf" | "document" | "worksheet" | "image" | "link" | "lesson_note"
        )
    {
        return HttpResponse::UnprocessableEntity()
            .json(serde_json::json!({"message":"Invalid resource details"}));
    };
    match sqlx::query("INSERT INTO classroom_resources(classroom_id,uploaded_by,name,file_url,resource_type) VALUES($1,$2,$3,$4,$5)").bind(cid).bind(id).bind(d.name.trim()).bind(d.file_url.trim()).bind(d.resource_type).execute(pool.get_ref()).await{Ok(_)=>HttpResponse::Created().json(serde_json::json!({"message":"Resource added"})),Err(_)=>HttpResponse::InternalServerError().finish()}
}
#[delete("/classrooms/{id}/resources/{resource_id}")]
pub async fn delete_resource(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<(Uuid, Uuid)>,
) -> impl Responder {
    let (cid, rid) = path.into_inner();
    let (_, role) = match authorize(pool.get_ref(), cid, &user).await {
        Ok(x) => x,
        Err(e) => return e,
    };
    if role != "tutor" {
        return HttpResponse::Forbidden().finish();
    };
    match sqlx::query("DELETE FROM classroom_resources WHERE id=$1 AND classroom_id=$2")
        .bind(rid)
        .bind(cid)
        .execute(pool.get_ref())
        .await
    {
        Ok(x) if x.rows_affected() == 1 => HttpResponse::NoContent().finish(),
        Ok(_) => HttpResponse::NotFound().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
#[get("/classrooms/{id}/attendance")]
pub async fn attendance(
    user: AuthUser,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
) -> impl Responder {
    let cid = path.into_inner();
    let (id, role) = match authorize(pool.get_ref(), cid, &user).await {
        Ok(x) => x,
        Err(e) => return e,
    };
    let q = if role == "tutor" {
        "SELECT a.user_id,CONCAT(u.first_name,' ',u.last_name) user_name,p.role,a.joined_at,a.left_at,a.duration_seconds FROM classroom_attendance a JOIN users u ON u.id=a.user_id JOIN classroom_participants p ON p.classroom_id=a.classroom_id AND p.user_id=a.user_id WHERE a.classroom_id=$1"
    } else {
        "SELECT a.user_id,CONCAT(u.first_name,' ',u.last_name) user_name,p.role,a.joined_at,a.left_at,a.duration_seconds FROM classroom_attendance a JOIN users u ON u.id=a.user_id JOIN classroom_participants p ON p.classroom_id=a.classroom_id AND p.user_id=a.user_id WHERE a.classroom_id=$1 AND a.user_id=$2"
    };
    match sqlx::query_as::<_, Attendance>(q)
        .bind(cid)
        .bind(id)
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(x) => HttpResponse::Ok().json(x),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
