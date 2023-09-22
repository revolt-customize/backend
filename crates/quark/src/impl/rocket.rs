use authifier::models::Session;
use authifier::Authifier;
use reqwest::header::COOKIE;
use revolt_models::v0;
use revolt_okapi::openapi3::{SecurityScheme, SecuritySchemeData};
use revolt_rocket_okapi::gen::OpenApiGenerator;
use revolt_rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome, Request};

use crate::models::User;
use crate::Database;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = authifier::Error;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let old_db = request.rocket().state::<Database>().expect("`Database`");

        let user: &Option<revolt_database::User> = request
            .local_cache_async(async {
                let db = request
                    .rocket()
                    .state::<revolt_database::Database>()
                    .expect("`Database`");

                let header_bot_token = request
                    .headers()
                    .get("x-bot-token")
                    .next()
                    .map(|x| x.to_string());

                if let Some(bot_token) = header_bot_token {
                    if let Ok(user) = revolt_database::User::from_token(db, &bot_token).await {
                        return Some(user);
                    }
                } else if let Outcome::Success(session) = request.guard::<Session>().await {
                    // This uses a guard so can't really easily be refactored into from_token at this stage.
                    if let Ok(user) = db.fetch_user(&session.user_id).await {
                        return Some(user);
                    }
                }

                None
            })
            .await;

        if let Some(user) = user {
            return Outcome::Success(old_db.fetch_user(&user.id).await.unwrap());
        }
        // else {
        //     return Outcome::Failure((Status::Unauthorized, authifier::Error::InvalidSession));
        // }

        // run uuap check
        let uuap_response: &Option<v0::UUAPResponse> = request
            .local_cache_async(async {
                let mut cookie_str = String::new();
                for cookie in request.cookies().iter() {
                    cookie_str.push_str(&format!("{}={}; ", cookie.name(), cookie.value()));
                }

                let config = revolt_config::config().await;

                let service_url = request.headers().get("Referer").next().unwrap_or("");
                let ticket = request.headers().get("Ticket").next().unwrap_or("");
                let url = format!(
                    "{}/v1/login?service={}&ticket={}",
                    config.api.botservice.chatall_server, service_url, ticket
                );

                let client = reqwest::Client::new();
                let response: v0::UUAPResponse = client
                    .get(url.clone())
                    .header(COOKIE, cookie_str)
                    .send()
                    .await
                    .expect("SendRequestFailed")
                    .json()
                    .await
                    .expect("ParseJsonFailed");

                Some(response)
            })
            .await;

        match &uuap_response.as_ref().unwrap().data {
            v0::UUAPResponseData::Forbidden { .. } => {
                return Outcome::Failure((Status::Forbidden, authifier::Error::InvalidInvite));
            }
            v0::UUAPResponseData::Redirect(..) => {
                return Outcome::Failure((Status::Unauthorized, authifier::Error::InvalidSession));
            }

            v0::UUAPResponseData::Success { user, .. } => {
                let authifier = request.rocket().state::<Authifier>().expect("`Authifier`");
                let db = request
                    .rocket()
                    .state::<revolt_database::Database>()
                    .expect("`Database`");
                let user = revolt_database::User::get_or_create_new_user(
                    authifier,
                    db,
                    user.username.clone(),
                    user.email.clone(),
                )
                .await;

                match user {
                    Ok((_, u)) => return Outcome::Success(old_db.fetch_user(&u.id).await.unwrap()),
                    Err(_) => {
                        return Outcome::Failure((
                            Status::InternalServerError,
                            authifier::Error::InvalidSession,
                        ))
                    }
                }
            }
        }
    }
}

impl<'r> OpenApiFromRequest<'r> for User {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> revolt_rocket_okapi::Result<RequestHeaderInput> {
        let mut requirements = schemars::Map::new();
        requirements.insert("Session Token".to_owned(), vec![]);

        Ok(RequestHeaderInput::Security(
            "Session Token".to_owned(),
            SecurityScheme {
                data: SecuritySchemeData::ApiKey {
                    name: "x-session-token".to_owned(),
                    location: "header".to_owned(),
                },
                description: Some("Used to authenticate as a user.".to_owned()),
                extensions: schemars::Map::new(),
            },
            requirements,
        ))
    }
}
