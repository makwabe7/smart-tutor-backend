use crate::middleware::auth::AuthUser;
use crate::models::centres::{CreateTuitionCentre, TuitionCentre};
use actix_web::{HttpResponse, Responder, get, post, web};
use sqlx::PgPool;
use uuid::Uuid;

fn slug_for(name: &str) -> String {
    let words = name
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let base = words
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    format!(
        "{}-{}",
        if base.is_empty() {
            "tuition-centre"
        } else {
            &base[..base.len().min(140)]
        },
        &Uuid::new_v4().to_string()[..8]
    )
}

#[get("/centres")]
pub async fn list_centres(pool: web::Data<PgPool>) -> impl Responder {
    let query = "SELECT id, slug, name, tagline, county, estate, address, description, subjects, grade_levels, monthly_fee, student_count, rating::float8 AS rating, review_count, image_url FROM tuition_centres WHERE is_active = TRUE ORDER BY rating DESC, name ASC";
    match sqlx::query_as::<_, TuitionCentre>(query)
        .fetch_all(pool.get_ref())
        .await
    {
        Ok(centres) => HttpResponse::Ok().json(centres),
        Err(error) => {
            eprintln!("Centre listing error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/centres/{slug}")]
pub async fn get_centre(pool: web::Data<PgPool>, slug: web::Path<String>) -> impl Responder {
    let query = "SELECT id, slug, name, tagline, county, estate, address, description, subjects, grade_levels, monthly_fee, student_count, rating::float8 AS rating, review_count, image_url FROM tuition_centres WHERE is_active = TRUE AND slug = $1";
    match sqlx::query_as::<_, TuitionCentre>(query)
        .bind(slug.into_inner())
        .fetch_optional(pool.get_ref())
        .await
    {
        Ok(Some(centre)) => HttpResponse::Ok().json(centre),
        Ok(None) => HttpResponse::NotFound()
            .json(serde_json::json!({"message": "Tuition centre not found"})),
        Err(error) => {
            eprintln!("Centre detail error: {error:?}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/centres")]
pub async fn create_centre(
    user: AuthUser,
    pool: web::Data<PgPool>,
    body: web::Json<CreateTuitionCentre>,
) -> impl Responder {
    let owner_id = match Uuid::parse_str(&user.user_id) {
        Ok(id) => id,
        Err(_) => return HttpResponse::Unauthorized().finish(),
    };
    let details = body.into_inner();
    if details.name.trim().len() < 3
        || details.county.trim().is_empty()
        || details.estate.trim().is_empty()
        || details.subjects.is_empty()
        || details.monthly_fee < 0
    {
        return HttpResponse::BadRequest().json(serde_json::json!({"message": "Add a centre name, location, subject, and valid monthly fee"}));
    }
    let slug = slug_for(details.name.trim());
    let query = r#"INSERT INTO tuition_centres (owner_id, slug, name, tagline, county, estate, address, description, subjects, grade_levels, monthly_fee, image_url)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING id, slug"#;
    match sqlx::query_as::<_, (Uuid, String)>(query).bind(owner_id).bind(&slug).bind(details.name.trim()).bind(details.tagline).bind(details.county.trim()).bind(details.estate.trim()).bind(details.address).bind(details.description).bind(details.subjects).bind(details.grade_levels).bind(details.monthly_fee).bind(details.image_url).fetch_one(pool.get_ref()).await {
        Ok((id, slug)) => HttpResponse::Created().json(serde_json::json!({"id": id, "slug": slug, "message": "Centre submitted for admin review"})),
        Err(error) => { eprintln!("Centre create error: {error:?}"); HttpResponse::InternalServerError().finish() }
    }
}
