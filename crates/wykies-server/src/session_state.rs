use actix_session::{Session, SessionExt, SessionGetError, SessionInsertError};
use actix_web::{FromRequest, HttpRequest, dev::Payload};
use std::future::{Ready, ready};
use wykies_shared::uac::UserInfo;

pub struct TypedSession(Session);

impl TypedSession {
    const USER_INFO_KEY: &'static str = "user_info";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn insert_user_info(&self, user_info: UserInfo) -> Result<(), SessionInsertError> {
        self.0.insert(Self::USER_INFO_KEY, user_info)
    }

    pub fn get_user_info(&self) -> Result<Option<UserInfo>, SessionGetError> {
        self.0.get(Self::USER_INFO_KEY)
    }

    pub fn log_out(self) {
        self.0.purge()
    }
}

impl FromRequest for TypedSession {
    type Error = <Session as FromRequest>::Error;
    type Future = Ready<Result<TypedSession, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
