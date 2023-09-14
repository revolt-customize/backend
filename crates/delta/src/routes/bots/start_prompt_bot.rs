use revolt_database::BotType;
use revolt_database::{util::reference::Reference, Database, User};
use revolt_result::{create_error, Result};
use rocket::serde::json::Json;
use rocket::State;

/// # Start a prompt Bot
///
/// Start a prompt bot in prompt server
#[openapi(tag = "Bots")]
#[post("/<bot>/start")]
pub async fn req(
    db: &State<Database>,
    user: User,
    bot: Reference,
) -> Result<Json<serde_json::Value>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let bot = bot.as_bot(db).await?;
    if bot.owner != user.id {
        return Err(create_error!(NotFound));
    }
    let config = revolt_config::config().await;
    if bot.bot_type != Some(BotType::PromptBot) && config.api.botservice.bot_server.is_empty() {
        return Err(create_error!(InvalidProperty));
    }

    let host = config.api.botservice.bot_server;
    let url = format!("{host}/api/rest/v1/bot/restart");
    let client = reqwest::Client::new();
    let response = client
        .get(url.clone())
        .query(&[("bot_token", &bot.token)])
        .send()
        .await
        .map_err(|_| create_error!(InternalError))?
        .json::<serde_json::Value>()
        .await
        .map_err(|_| create_error!(InternalError))?;
    Ok(Json(response))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{Bot, BotType, PartialBot};
    use revolt_models::v0;
    use rocket::http::{ContentType, Header, Status};

    #[rocket::async_test]
    async fn start_bot() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let mut owner = user.clone();
        let bot_information = v0::BotInformation {
            owner_id: owner.id.clone(),
            model: Some(Default::default()),
        };

        owner.bot = Some(bot_information.into());
        let bot = Bot::create(
            &harness.db,
            TestHarness::rand_string(),
            &owner,
            PartialBot {
                bot_type: Some(BotType::PromptBot),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let response = harness
            .client
            .post(format!("/bots/{}/start", bot.id))
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(ContentType::JSON)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let resp = response.into_json::<serde_json::Value>().await;
        info!("{:?}", resp);
    }
}
