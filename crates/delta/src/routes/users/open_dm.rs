use revolt_quark::{
    models::{Channel, User},
    Database, Ref, Result,
};

use rocket::{serde::json::Json, State};
use ulid::Ulid;

/// # Open Direct Message
///
/// Open a DM with another user.
///
/// If the target is oneself, a saved messages channel is returned.
#[openapi(tag = "Direct Messaging")]
#[get("/<target>/dm")]
pub async fn req(db: &State<Database>, user: User, target: Ref) -> Result<Json<Channel>> {
    let target = target.as_user(db).await?;

    // If the target is oneself, open saved messages.
    if target.id == user.id {
        return if let Ok(channel) = db.find_direct_message_channel(&user.id, &target.id).await {
            Ok(Json(channel))
        } else {
            let new_channel = Channel::SavedMessages {
                id: Ulid::new().to_string(),
                user: user.id,
            };

            new_channel.create(db).await?;
            Ok(Json(new_channel))
        };
    }

    // Otherwise try to find or create a DM.
    if let Ok(channel) = db.find_direct_message_channel(&user.id, &target.id).await {
        return Ok(Json(channel));
    }

    let new_channel = Channel::DirectMessage {
        id: Ulid::new().to_string(),
        active: false,
        recipients: vec![user.id, target.id],
        last_message_id: None,
    };

    new_channel.create(db).await?;
    Ok(Json(new_channel))
}

#[cfg(test)]
mod tests {
    use revolt_database::Bot;
    use revolt_quark::models::Channel;
    use rocket::http::{Header, Status};

    use crate::util::test::TestHarness;

    #[rocket::async_test]
    async fn remove_backgroud_profile() {
        let harness = TestHarness::new().await;
        let (_, session, from) = harness.new_user().await;

        let (_, _, user) = harness.new_user().await;
        let bot_name = TestHarness::rand_string();
        let bot = Bot::create(&harness.db, bot_name.clone(), &user, None)
            .await
            .expect("`Bot`");

        let response = harness
            .client
            .get(format!("/users/{}/dm", bot.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        // println!("{:?}", response.into_string().await);
        assert_eq!(response.status(), Status::Ok);

        let channel = response.into_json::<Channel>().await.unwrap();
        match channel {
            Channel::DirectMessage {
                active, recipients, ..
            } => {
                assert!(!active);
                assert_eq!(recipients, vec![from.id, bot.id])
            }
            _ => unreachable!(),
        }
    }
}
