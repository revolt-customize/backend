use std::io::Cursor;

use revolt_models::v0;
use revolt_result::create_error;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{ContentType, Status};
use rocket::{Request, Response};

/// Attach UserAuthFairing to the Rocket application
pub struct UserAuthFairing;

#[rocket::async_trait]
impl Fairing for UserAuthFairing {
    fn info(&self) -> Info {
        Info {
            name: "UserAuthFairing",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        log::info!("catch fairing");
        let uuap_response: &Option<v0::UUAPResponse> = request.local_cache(|| None);
        log::info!("got uuap : {uuap_response:?}");

        if let Some(uuap) = uuap_response {
            match &uuap.data {
                v0::UUAPResponseData::Redirect(uri) => {
                    response.set_status(Status::Found);
                    response.set_raw_header("X-Location", uri);
                }

                v0::UUAPResponseData::Forbidden { username } => {
                    response.set_status(Status::Forbidden);

                    let string = serde_json::to_string(&create_error!(ForbiddenUser {
                        username: username.to_string()
                    }))
                    .unwrap();
                    response.set_sized_body(string.len(), Cursor::new(string));
                    response.set_header(ContentType::new("application", "json"));
                }

                v0::UUAPResponseData::Success { cookie, .. } => {
                    let mut cookie_str = String::new();
                    for (key, value) in cookie {
                        cookie_str.push_str(&format!("{}={}; ", key, value));
                    }
                    response.set_raw_header("Set-Cookie", cookie_str);
                }
            }
        }
    }
}
