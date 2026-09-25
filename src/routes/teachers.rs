use crate::handlers::availability::{
    available_slots, create_availability, delete_availability, public_availability,
    update_availability,
};
use crate::handlers::bookings::teacher_bookings;
use crate::handlers::teacher::{
    dashboard, request_payout, update_booking_status, update_profile, wallet,
};
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/teacher")
            .service(update_profile)
            .service(dashboard)
            .service(wallet)
            .service(request_payout)
            .service(update_booking_status)
            .service(teacher_bookings),
    );
    cfg.service(
        web::scope("/api/teachers")
            .service(create_availability)
            .service(update_availability)
            .service(delete_availability)
            .service(public_availability)
            .service(available_slots),
    );
}
