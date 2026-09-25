use crate::handlers::admin::{
    approve_centre_submission, dashboard_metrics, delete_user, get_settings,
    list_centre_submissions, list_logs, list_tutor_verifications, list_users, update_active,
    update_role, update_settings, update_tutor_verification,
};
use actix_web::web;
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(dashboard_metrics)
        .service(list_users)
        .service(update_role)
        .service(update_active)
        .service(delete_user)
        .service(list_logs)
        .service(list_tutor_verifications)
        .service(update_tutor_verification)
        .service(list_centre_submissions)
        .service(approve_centre_submission)
        .service(get_settings)
        .service(update_settings);
}
