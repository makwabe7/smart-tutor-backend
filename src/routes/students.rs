use actix_web::web;

use crate::handlers::bookings::{
    booking_detail, cancel_booking, complete_booking, confirm_booking, create_booking,
    student_bookings,
};
use crate::handlers::searches::{recommendations, record_search};
use crate::handlers::student::{dashboard, get_profile, update_profile};
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/student")
            .service(dashboard)
            .service(get_profile)
            .service(update_profile),
    )
    .service(
        web::scope("/api/student")
            .service(record_search)
            .service(recommendations)
            .service(create_booking),
    )
    .service(
        web::scope("/api")
            .service(student_bookings)
            .service(booking_detail)
            .service(cancel_booking)
            .service(confirm_booking)
            .service(complete_booking),
    );
}
