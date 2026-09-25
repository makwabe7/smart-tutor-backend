use crate::handlers::auth::{
    confirm_password_reset, login, me, register, request_password_reset, request_phone_login,
    resend_login_verification, verify_login, verify_phone_login,
};
use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/auth")
            .service(register)
            .service(login)
            .service(me)
            .service(verify_login)
            .service(resend_login_verification)
            .service(request_phone_login)
            .service(verify_phone_login)
            .service(request_password_reset)
            .service(confirm_password_reset),
    );
}
