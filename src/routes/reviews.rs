use crate::handlers::reviews::{create_review, list_reviews};
use actix_web::web;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_reviews).service(create_review);
}
