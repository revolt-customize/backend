use authifier::models::Session;
use reqwest::header::COOKIE;
use revolt_result::{Error, ErrorType};
use rocket::http::Status;

use authifier::models::Account;
use authifier::Authifier;
use revolt_models::v0;
use rocket::request::{self, FromRequest, Outcome, Request};

use crate::{Database, User};

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = authifier::Error;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let user: &Option<User> = request
            .local_cache_async(async {
                let db = request.rocket().state::<Database>().expect("`Database`");

                let header_bot_token = request
                    .headers()
                    .get("x-bot-token")
                    .next()
                    .map(|x| x.to_string());

                if let Some(bot_token) = header_bot_token {
                    if let Ok(user) = User::from_token(db, &bot_token).await {
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
            return Outcome::Success(user.clone());
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
                let db = request.rocket().state::<Database>().expect("`Database`");
                let user = User::get_or_create_new_user(
                    authifier,
                    db,
                    user.username.clone(),
                    user.email.clone(),
                )
                .await;

                match user {
                    Ok((_, u)) => return Outcome::Success(u),
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

impl User {
    pub async fn get_or_create_new_user(
        authifier: &Authifier,
        db: &Database,
        username: String,
        email: String,
    ) -> Result<(Account, User), Error> {
        // fetch account by email
        if let Some(account) = authifier
            .database
            .find_account_by_normalised_email(&email)
            .await
            .expect("GetAccountFailed")
        {
            // account exist
            match db.fetch_user(&account.id).await {
                Err(e) => {
                    if let ErrorType::NotFound = e.error_type {
                        // create a new user if not found user
                        let user = User::create(db, username.clone(), account.id.to_string(), None)
                            .await
                            .expect("`User`");

                        return Ok((account, user));
                    }

                    Err(create_error!(InternalError))
                }

                Ok(u) => Ok((account, u)),
            }
        } else {
            // account not exist, create a new account and user
            let (account, user) = new_user(authifier, db, username.clone(), email.clone()).await;
            Ok((account, user))
        }
    }
}

// create a new user
async fn new_user(
    authifier: &Authifier,
    db: &Database,
    username: String,
    email: String,
) -> (Account, User) {
    let account = Account::new(authifier, email.clone(), email.clone(), false)
        .await
        .expect("`Account`");
    let user = User::create(db, username, account.id.to_string(), None)
        .await
        .expect("`User`");

    User::prepare_on_board_data(db, user.id.clone())
        .await
        .expect("PrepareOnBoardData");

    (account, user)
}
