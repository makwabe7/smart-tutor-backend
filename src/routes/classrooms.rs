use crate::handlers::classrooms::*;
use actix_web::web;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .service(classroom_socket)
            .service(list_classrooms)
            .service(get_classroom)
            .service(get_booking_classroom)
            .service(join_classroom)
            .service(leave_classroom)
            .service(start_classroom)
            .service(end_classroom)
            .service(list_messages)
            .service(send_message)
            .service(list_resources)
            .service(create_resource)
            .service(delete_resource)
            .service(attendance),
    );
}
