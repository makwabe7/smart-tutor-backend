use crate::handlers::centres::{create_centre, get_centre, list_centres};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_centres)
        .service(get_centre)
        .service(create_centre);
}
