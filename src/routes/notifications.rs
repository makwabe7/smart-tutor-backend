use actix_web::web;

use crate::handlers::notifications::{list, mark_all_read, mark_read};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/notifications")
            .service(list)
            .service(mark_read)
            .service(mark_all_read),
    );
}
