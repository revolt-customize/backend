use revolt_config::config;
use revolt_database::{Channel, Database, RelationshipStatus, User};
use revolt_models::v0;
use revolt_result::{create_error, Result};
use rocket::serde::json::Json;
use rocket::State;
use validator::Validate;

/// # Create Group
///
/// Create a new group channel.
#[openapi(tag = "Groups")]
#[post("/create", data = "<data>")]
pub async fn create_group(
    db: &State<Database>,
    user: User,
    data: Json<v0::DataCreateGroup>,
) -> Result<Json<v0::Channel>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let mut data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    for target in &data.users {
        match user.relationship_with(target) {
            RelationshipStatus::Friend | RelationshipStatus::User => {}
            _ => {
                return Err(create_error!(NotFriends));
            }
        }
    }

    let config = config().await;
    if !config.api.botservice.official_model_bots.is_empty() {
        data.users
            .extend(config.api.botservice.official_model_bots.iter().cloned());
    }

    Ok(Json(Channel::create_group(db, data, user.id).await?.into()))
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::{rocket, util::test::TestHarness};
    use revolt_database::events::client::EventV1;
    use revolt_models::v0;
    use rocket::http::{ContentType, Header, Status};

    #[rocket::async_test]
    async fn create_group() {
        let mut harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let response = harness
            .client
            .post("/channels/create")
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(ContentType::JSON)
            .body(
                json!(v0::DataCreateGroup {
                    name: TestHarness::rand_string(),
                    description: None,
                    icon: None,
                    users: HashSet::new(),
                    nsfw: None
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);

        let channel: v0::Channel = response.into_json().await.expect("`Channel`");
        match channel {
            v0::Channel::Group {
                id,
                owner,
                recipients,
                ..
            } => {
                assert_eq!(owner, user.id);
                assert_eq!(recipients.len(), 3);
                assert!(harness.db.fetch_channel(&id).await.is_ok());

                let event = harness
                    .wait_for_event(&format!("{}!", user.id), |event| match event {
                        EventV1::ChannelCreate(channel) => channel.id() == id,
                        _ => false,
                    })
                    .await;

                match event {
                    EventV1::ChannelCreate(v0::Channel::Group {
                        owner: channel_owner,
                        ..
                    }) => {
                        assert_eq!(owner, channel_owner);
                    }
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }
}
