use once_cell::sync::Lazy;
use regex::Regex;
use revolt_database::{Database, User};
use revolt_models::v0;
use revolt_quark::authifier::models::Session;
use revolt_result::{create_error, Result};

use rocket::{serde::json::Json, State};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Regex for valid usernames
///
/// Block zero width space
/// Block lookalike characters
pub static RE_USERNAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\p{L}|[\d_.-])+$").unwrap());

/// # New User Data
#[derive(Validate, Serialize, Deserialize, JsonSchema)]
pub struct DataOnboard {
    /// New username which will be used to identify the user on the platform
    #[validate(length(min = 2, max = 32), regex = "RE_USERNAME")]
    username: String,
}

/// # Complete Onboarding
///
/// This sets a new username, completes onboarding and allows a user to start using Revolt.
#[openapi(tag = "Onboarding")]
#[post("/complete", data = "<data>")]
pub async fn req(
    db: &State<Database>,
    session: Session,
    user: Option<User>,
    data: Json<DataOnboard>,
) -> Result<Json<v0::User>> {
    if user.is_some() {
        return Err(create_error!(AlreadyOnboarded));
    }

    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;
    let new_user = User::create(db, data.username, session.user_id, None).await?;

    User::prepare_on_board_data(db, new_user.id.clone()).await?;

    Ok(Json(new_user.into_self().await))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{rocket, routes::onboard::complete::DataOnboard, util::test::TestHarness};
    use revolt_database::Channel;
    use revolt_models::v0;
    use rocket::http::{ContentType, Header, Status};

    #[rocket::async_test]
    async fn test_on_board_compelete() {
        let harness = TestHarness::new().await;
        let (_, session) = harness.new_account_session().await;

        let config = revolt_config::config().await;

        let response = harness
            .client
            .post("/onboard/complete")
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(ContentType::JSON)
            .body(
                json!(DataOnboard {
                    username: "cac".into()
                })
                .to_string(),
            )
            .dispatch()
            .await;
        let status = response.status();
        // println!("{:}", response.into_string().await.unwrap());
        assert_eq!(status, Status::Found);

        let user = response.into_json::<v0::User>().await.unwrap();
        let channels = harness.db.find_direct_messages(&user.id).await.unwrap();

        assert_eq!(channels.len(), 3);

        let mut match_cnt = 0;

        for channel in channels.into_iter() {
            match channel {
                Channel::Group {
                    owner, recipients, ..
                } => {
                    assert_eq!(owner, user.id);
                    let set: HashSet<String> = recipients.into_iter().collect();
                    let mut expect = HashSet::new();
                    expect.insert(user.id.clone());
                    for id in config.api.botservice.official_model_bots.as_slice() {
                        expect.insert(id.clone());
                    }
                    assert_eq!(set, expect);
                    match_cnt += 1;
                }

                Channel::DirectMessage { .. } => {
                    match_cnt += 1;
                }
                _ => panic!("error"),
            }
        }

        assert_eq!(3, match_cnt);
    }
}
