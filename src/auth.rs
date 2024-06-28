use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use rocket::{async_trait, Request};
use rocket::http::Status;
use rocket::outcome::Outcome::Forward;
use rocket::request::{FromRequest, Outcome};
use crate::ARProxyConfiguration;

pub struct ApiCredentials {
    pub user: String,
    pub key: String,
}


#[async_trait]
impl<'r> FromRequest<'r> for &'r ApiCredentials {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_header = if let Some(x) = request.headers().get("Authorization")
            .next() {
            x
        } else {
            return Forward(Status::Unauthorized);
        };

        let auth_header = if let Some(result) =  auth_header.strip_prefix("Basic ") {
            result
        } else {
            return Forward(Status::Unauthorized)
        };

        let auth_header = if let Ok(header) = BASE64_STANDARD
            .decode(auth_header) {
            header
        } else {
            return Forward(Status::Unauthorized);
        };

        let colon_index = if let Some(pos) = auth_header.iter().position(|elem| elem == &b':') {
            pos
        } else {
            return Forward(Status::Unauthorized);
        };

        let (user, mut key) = auth_header
            .split_at(colon_index);

        key = &key[1..];

        let config = request.rocket().state::<ARProxyConfiguration>().unwrap();

        if config.creds.user.as_bytes() == user && config.creds.key.as_bytes() == key {
            Outcome::Success(&config.creds)
        } else {
            Forward(Status::Unauthorized)
        }
    }
}