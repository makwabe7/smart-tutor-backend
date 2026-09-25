use crate::middleware::auth::AuthUser;
use actix_web::{Error, FromRequest, HttpRequest, dev::Payload, error::ErrorForbidden};
use futures_util::future::{Ready, ready};
pub struct Student(pub AuthUser);

impl FromRequest for Student {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let auth = match AuthUser::from_request(req, payload).into_inner() {
            Ok(user) => user,
            Err(error) => return ready(Err(error)),
        };
        if auth.role != "student" {
            return ready(Err(ErrorForbidden(
                "Unauthorized .Student access denied.".to_string(),
            )));
        }
        ready(Ok(Student(auth)))
    }
}
