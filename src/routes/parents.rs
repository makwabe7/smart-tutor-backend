use crate::handlers::parents::{
    accept_link_request, add_managed_child, list_children, list_link_requests, request_student_link,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/parent")
            .service(add_managed_child)
            .service(request_student_link)
            .service(list_children),
    );
    cfg.service(
        web::scope("/api/student/parent-links")
            .service(list_link_requests)
            .service(accept_link_request),
    );
}
