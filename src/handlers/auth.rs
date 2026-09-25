use crate::middleware::auth::AuthUser;
use actix_web::{HttpResponse, Responder, get, post, web};
use bcrypt::{DEFAULT_COST, hash, verify};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, query_scalar};
use std::env;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ChallengeRequest {
    pub challenge_id: Uuid,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct ResendChallengeRequest {
    pub challenge_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetRequest {
    pub identifier: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetConfirm {
    pub challenge_id: Uuid,
    pub code: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct PhoneLoginRequest {
    pub phone: String,
}

#[derive(sqlx::FromRow)]
struct LoginUser {
    id: Uuid,
    email: String,
    first_name: String,
    last_name: String,
    role: String,
    password_hash: Option<String>,
    is_active: bool,
}

#[derive(sqlx::FromRow)]
struct ChallengeRow {
    id: Uuid,
    user_id: Uuid,
    code_hash: String,
}

fn one_time_code() -> String {
    format!("{:06}", Uuid::new_v4().as_u128() % 1_000_000)
}

fn expose_development_codes() -> bool {
    cfg!(debug_assertions) || env::var("AUTH_EXPOSE_DEV_CODES").ok().as_deref() == Some("true")
}

fn normalize_kenyan_phone(phone: &str) -> Option<String> {
    let digits: String = phone
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect();
    match digits.as_str() {
        value
            if value.len() == 10
                && value.starts_with('0')
                && value.as_bytes().get(1) == Some(&b'7') =>
        {
            Some(format!("254{}", &value[1..]))
        }
        value if value.len() == 9 && value.starts_with('7') => Some(format!("254{value}")),
        value if value.len() == 12 && value.starts_with("2547") => Some(value.to_owned()),
        _ => None,
    }
}

fn challenge_response(challenge_id: Uuid, code: String, message: &str) -> HttpResponse {
    let mut response = serde_json::json!({"challenge_id": challenge_id, "message": message});
    if expose_development_codes() {
        response["dev_code"] = serde_json::Value::String(code);
    }
    HttpResponse::Ok().json(response)
}

async fn create_challenge(
    pool: &PgPool,
    user_id: Uuid,
    purpose: &str,
) -> Result<(Uuid, String), sqlx::Error> {
    let code = one_time_code();
    let code_hash = hash(&code, DEFAULT_COST)
        .map_err(|_| sqlx::Error::Protocol("Could not hash challenge".into()))?;
    let id = sqlx::query_scalar::<_, Uuid>("INSERT INTO auth_challenges (user_id, purpose, code_hash, expires_at) VALUES ($1,$2,$3,NOW() + INTERVAL '10 minutes') RETURNING id")
        .bind(user_id).bind(purpose).bind(code_hash).fetch_one(pool).await?;
    Ok((id, code))
}

async fn issue_token(pool: &PgPool, user_id: Uuid) -> Result<HttpResponse, HttpResponse> {
    let user = sqlx::query_as::<_, LoginUser>("SELECT id,email,password_hash,first_name,last_name,role::text AS role, is_active FROM users WHERE id = $1").bind(user_id).fetch_one(pool).await.map_err(|_| HttpResponse::InternalServerError().json(serde_json::json!({"message":"Could not load account"})))?;
    let secret = env::var("JWT_SECRET").map_err(|_| {
        HttpResponse::InternalServerError()
            .json(serde_json::json!({"message":"JWT configuration missing"}))
    })?;
    let claims = Claims {
        sub: user.id.to_string(),
        role: user.role.clone(),
        exp: chrono::Utc::now().timestamp() as usize + 24 * 60 * 60,
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| HttpResponse::InternalServerError().finish())?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message":"successfully logged in","token":token,"user":{"id":user.id,"email":user.email,"first_name":user.first_name,"last_name":user.last_name,"role":user.role}})))
}
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub phone: Option<String>,
    pub password_hash: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
}
#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub message: String,
    pub user_id: uuid::Uuid,
}

#[post("/register")]
pub async fn register(pool: web::Data<PgPool>, data: web::Json<RegisterRequest>) -> impl Responder {
    // Administrative roles must never be self-service.  They are assigned by
    // an existing super administrator through the admin API after a normal
    // account has been created.
    let role = match data.role.as_str() {
        "student" | "parent" | "teacher" => data.role.as_str(),
        _ => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "message": "Choose a valid account role"
            }));
        }
    };
    let password_hash = match hash(&data.password_hash, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(_) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Failed tp process password hash"
            }));
        }
    };
    let result = query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (
        email,
        phone,
        password_hash,
        first_name,
        last_name,
        role
        )
        VALUES ($1, $2, $3, $4, $5, $6::user_role)
        RETURNING id
        "#,
    )
    .bind(&data.email)
    .bind(data.phone.as_deref().and_then(normalize_kenyan_phone))
    .bind(&password_hash)
    .bind(&data.first_name)
    .bind(&data.last_name)
    .bind(role)
    .fetch_one(pool.get_ref())
    .await;
    match result {
        Ok(user) => {
            // A registered teacher must have a public-profile record so that the
            // live tutor directory can discover them. The profile is intentionally
            // empty until the tutor completes their teaching details.
            if role == "teacher" {
                if let Err(error) = sqlx::query("INSERT INTO teacher_profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING")
                    .bind(user)
                    .execute(pool.get_ref())
                    .await
                {
                    eprintln!("Could not create teacher profile shell: {error:?}");
                    return HttpResponse::InternalServerError().json(serde_json::json!({"message": "Could not prepare teaching profile"}));
                }
            }
            let _ = sqlx::query(
                "INSERT INTO user_notifications (user_id, title, body, category, href) VALUES ($1, $2, $3, 'system', $4)",
            )
            .bind(user)
            .bind("Welcome to Somo!")
            .bind("Your account is ready. Complete your profile to get the most from Somo.")
            .bind("/dashboard")
            .execute(pool.get_ref())
            .await;
            HttpResponse::Created().json(RegisterResponse {
                message: "successfully registered".to_string(),
                user_id: user,
            })
        }
        Err(error) => {
            eprintln!("Registration error: {:?}", error);
            HttpResponse::BadRequest().json(serde_json::json!({
                "message": "Failed register user"
            }))
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
}
#[post("/login")]
pub async fn login(pool: web::Data<PgPool>, data: web::Json<LoginRequest>) -> impl Responder {
    let user = sqlx::query_as::<_, LoginUser>("SELECT id,email,password_hash,first_name,last_name,role::text AS role, is_active FROM users WHERE email = $1")
        .bind(&data.email).fetch_optional(pool.get_ref()).await;
    let user = match user {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "message": "Invalid email or Password"
            }));
        }
        Err(error) => {
            eprintln!("Login database error:{:?}", error);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Database error"
            }));
        }
    };
    let password_valid = match verify(&data.password, user.password_hash.as_deref().unwrap_or("")) {
        Ok(valid) => valid,
        Err(_) => false,
    };
    if !password_valid {
        return HttpResponse::Unauthorized().json(serde_json::json!({
            "message": "Invalid Email or Password"
        }));
    }
    if !user.is_active {
        return HttpResponse::Forbidden().json(serde_json::json!({"message": "This account has been suspended. Contact a platform administrator."}));
    }
    let (challenge_id, code) = match create_challenge(pool.get_ref(), user.id, "login_2fa").await {
        Ok(challenge) => challenge,
        Err(error) => {
            eprintln!("2FA challenge error: {error:?}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"message":"Could not start two-factor verification"}));
        }
    };
    let mut response = serde_json::json!({"requires_2fa":true,"challenge_id":challenge_id,"message":"Enter the verification code"});
    if expose_development_codes() {
        response["dev_code"] = serde_json::Value::String(code);
    }
    HttpResponse::Ok().json(response)
}

#[post("/login/verify")]
pub async fn verify_login(
    pool: web::Data<PgPool>,
    body: web::Json<ChallengeRequest>,
) -> impl Responder {
    let row = sqlx::query_as::<_, ChallengeRow>("SELECT id,user_id,code_hash FROM auth_challenges WHERE id=$1 AND purpose='login_2fa' AND consumed_at IS NULL AND expires_at > NOW()").bind(body.challenge_id).fetch_optional(pool.get_ref()).await;
    let row = match row {
        Ok(Some(row)) => row,
        _ => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({"message":"Code expired or invalid"}));
        }
    };
    if !verify(&body.code, &row.code_hash).unwrap_or(false) {
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"message":"Code expired or invalid"}));
    }
    let _ = sqlx::query("UPDATE auth_challenges SET consumed_at=NOW() WHERE id=$1")
        .bind(row.id)
        .execute(pool.get_ref())
        .await;
    match issue_token(pool.get_ref(), row.user_id).await {
        Ok(response) => response,
        Err(response) => response,
    }
}

#[post("/login/verify/resend")]
pub async fn resend_login_verification(
    pool: web::Data<PgPool>,
    body: web::Json<ResendChallengeRequest>,
) -> impl Responder {
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM auth_challenges WHERE id=$1 AND purpose='login_2fa' AND consumed_at IS NULL AND expires_at > NOW()")
        .bind(body.challenge_id).fetch_optional(pool.get_ref()).await;
    let user_id = match user_id {
        Ok(Some(id)) => id,
        _ => return HttpResponse::Unauthorized().json(
            serde_json::json!({"message":"Verification session expired. Please sign in again."}),
        ),
    };
    let _ = sqlx::query("UPDATE auth_challenges SET consumed_at=NOW() WHERE id=$1")
        .bind(body.challenge_id)
        .execute(pool.get_ref())
        .await;
    match create_challenge(pool.get_ref(), user_id, "login_2fa").await {
        Ok((id, code)) => challenge_response(id, code, "A new verification code has been sent."),
        Err(error) => {
            eprintln!("2FA resend error: {error:?}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"message":"Could not send a new verification code"}))
        }
    }
}

#[post("/phone-login/request")]
pub async fn request_phone_login(
    pool: web::Data<PgPool>,
    body: web::Json<PhoneLoginRequest>,
) -> impl Responder {
    let phone = match normalize_kenyan_phone(&body.phone) {
        Some(phone) => phone,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"message":"Enter a valid Kenyan phone number"}));
        }
    };
    let user = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE phone=$1")
        .bind(phone)
        .fetch_optional(pool.get_ref())
        .await;
    let user = match user {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({"message":"No account exists for this phone number"}));
        }
        Err(error) => {
            eprintln!("Phone login lookup error: {error:?}");
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"message":"Could not start phone sign-in"}));
        }
    };
    match create_challenge(pool.get_ref(), user, "phone_login").await {
        Ok((id, code)) => challenge_response(id, code, "A sign-in code has been sent."),
        Err(error) => {
            eprintln!("Phone login challenge error: {error:?}");
            HttpResponse::InternalServerError()
                .json(serde_json::json!({"message":"Could not start phone sign-in"}))
        }
    }
}

#[post("/phone-login/verify")]
pub async fn verify_phone_login(
    pool: web::Data<PgPool>,
    body: web::Json<ChallengeRequest>,
) -> impl Responder {
    let row = sqlx::query_as::<_, ChallengeRow>("SELECT id,user_id,code_hash FROM auth_challenges WHERE id=$1 AND purpose='phone_login' AND consumed_at IS NULL AND expires_at > NOW()").bind(body.challenge_id).fetch_optional(pool.get_ref()).await;
    let row = match row {
        Ok(Some(row)) => row,
        _ => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({"message":"Code expired or invalid"}));
        }
    };
    if !verify(&body.code, &row.code_hash).unwrap_or(false) {
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"message":"Code expired or invalid"}));
    }
    let _ = sqlx::query("UPDATE auth_challenges SET consumed_at=NOW() WHERE id=$1")
        .bind(row.id)
        .execute(pool.get_ref())
        .await;
    match issue_token(pool.get_ref(), row.user_id).await {
        Ok(response) => response,
        Err(response) => response,
    }
}

#[post("/password-reset/request")]
pub async fn request_password_reset(
    pool: web::Data<PgPool>,
    body: web::Json<PasswordResetRequest>,
) -> impl Responder {
    let identifier = body.identifier.trim();
    let normalized_phone = normalize_kenyan_phone(identifier);
    let user = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM users WHERE LOWER(email)=LOWER($1) OR phone=$2",
    )
    .bind(identifier)
    .bind(normalized_phone)
    .fetch_optional(pool.get_ref())
    .await;
    let user = match user {
        Ok(Some(id)) => id,
        _ => {
            return HttpResponse::Ok().json(
                serde_json::json!({"message":"If the account exists, a code has been sent."}),
            );
        }
    };
    match create_challenge(pool.get_ref(), user, "password_reset").await {
        Ok((id, code)) => {
            challenge_response(id, code, "If the account exists, a code has been sent.")
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[post("/password-reset/confirm")]
pub async fn confirm_password_reset(
    pool: web::Data<PgPool>,
    body: web::Json<PasswordResetConfirm>,
) -> impl Responder {
    if body.new_password.len() < 8 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"message":"Password must be at least 8 characters"}));
    }
    let row = sqlx::query_as::<_, ChallengeRow>("SELECT id,user_id,code_hash FROM auth_challenges WHERE id=$1 AND purpose='password_reset' AND consumed_at IS NULL AND expires_at > NOW()").bind(body.challenge_id).fetch_optional(pool.get_ref()).await;
    let row = match row {
        Ok(Some(row)) => row,
        _ => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({"message":"Code expired or invalid"}));
        }
    };
    if !verify(&body.code, &row.code_hash).unwrap_or(false) {
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"message":"Code expired or invalid"}));
    }
    let password_hash = match hash(&body.new_password, DEFAULT_COST) {
        Ok(value) => value,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let _ = sqlx::query("UPDATE users SET password_hash=$1, updated_at=NOW() WHERE id=$2")
        .bind(password_hash)
        .bind(row.user_id)
        .execute(pool.get_ref())
        .await;
    let _ = sqlx::query("UPDATE auth_challenges SET consumed_at=NOW() WHERE id=$1")
        .bind(row.id)
        .execute(pool.get_ref())
        .await;
    HttpResponse::Ok().json(serde_json::json!({"message":"Password updated. You can sign in now."}))
}
#[get("/me")]
pub async fn me(user: AuthUser) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "user_id": user.user_id,
        "role": user.role
    }))
}
