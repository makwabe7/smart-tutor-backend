mod db;
mod handlers;
mod middleware;
mod models;
mod routes;
use actix_cors::Cors;
use actix_web::{App, HttpServer, Responder, get, web};
use dotenvy::dotenv;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = db::create_pool(&database_url)
        .await
        .expect("Error creating database pool");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Error running database migrations");
    println!("Connected to postgres");
    println!("Soma server running on http://127.0.0.1:8080");
    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allowed_origin("http://localhost:5173")
                    .allowed_origin("http://127.0.0.1:5173")
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![
                        actix_web::http::header::AUTHORIZATION,
                        actix_web::http::header::CONTENT_TYPE,
                    ])
                    .max_age(3600),
            )
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::JsonConfig::default().limit(2 * 1024 * 1024))
            .service(home)
            .service(health)
            // Keep the public marketplace routes at the application level.
            // This guarantees the URLs consumed by the Vite client do not
            // depend on nested-scope matching.
            .service(handlers::teacher::list_tutors)
            .service(handlers::teacher::get_tutor_profile)
            .service(handlers::teacher::public_schedule)
            .service(handlers::teacher::dashboard)
            .service(handlers::teacher::update_profile)
            .service(handlers::teacher::update_booking_status)
            .service(handlers::teacher::wallet)
            .service(handlers::teacher::request_payout)
            .service(handlers::availability::available_slots)
            .service(handlers::bookings::create_booking)
            .configure(routes::auth::config)
            .configure(routes::students::configure)
            .configure(routes::teachers::configure)
            .configure(routes::reviews::configure)
            .configure(routes::admin::configure)
            .configure(routes::notifications::configure)
            .configure(routes::parents::configure)
            .configure(routes::centres::configure)
            .configure(routes::classrooms::configure)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
#[get("/")]
async fn home() -> impl Responder {
    "Soma app is running"
}
#[get("/api/health")]
async fn health() -> impl Responder {
    "Soma app is healthy"
}
