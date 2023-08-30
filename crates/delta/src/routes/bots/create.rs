use revolt_database::{Bot, BotType, Database, PartialBot, User};
use revolt_models::v0;
use revolt_quark::variables::delta::BOT_SERVER_PUBLIC_URL;
use revolt_result::{create_error, Result};
use rocket::serde::json::Json;
use rocket::State;
use validator::Validate;

#[derive(Debug, serde::Serialize)]
struct CreatePromptBotReq {
    user_id: String,
    user_name: String,
    bot_id: String,
    bot_name: String,
    bot_token: String,
    model_name: String,
    prompt_template: String,
    temperature: f32,
}

/// # Create Bot
///
/// Create a new Revolt bot.
#[openapi(tag = "Bots")]
#[post("/create", data = "<info>")]
pub async fn create_bot(
    db: &State<Database>,
    user: User,
    info: Json<v0::DataCreateBot>,
) -> Result<Json<v0::Bot>> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let info = info.into_inner();
    info.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let mut owner = user.clone();

    let mut bot_information = v0::BotInformation {
        owner_id: owner.id.clone(),
        model: None,
    };

    let mut bot_type = BotType::CustomBot;

    if let Some(v0::BotType::PromptBot) = info.bot_type {
        bot_type = BotType::PromptBot;
        match info.model {
            Some(m) => bot_information.model = Some(m),
            None => {
                bot_information.model = Some(Default::default());
            }
        }
    }

    owner.bot = Some(bot_information.clone().into());

    let bot = Bot::create(
        db,
        info.name.clone(),
        &owner,
        PartialBot {
            bot_type: Some(bot_type.clone()),
            ..Default::default()
        },
    )
    .await?;

    if bot_type == BotType::PromptBot && !(*BOT_SERVER_PUBLIC_URL).is_empty() {
        let model = bot_information.model.unwrap_or(Default::default());

        let data = CreatePromptBotReq {
            user_id: bot.owner.clone(),
            user_name: user.username.clone(),
            bot_id: bot.id.clone(),
            bot_name: info.name,
            bot_token: bot.token.clone(),
            model_name: model.model_name,
            prompt_template: model.prompts.system_prompt,
            temperature: model.temperature,
        };

        let host = BOT_SERVER_PUBLIC_URL.to_string();
        let url = format!("{host}/api/rest/v1/bot/create");
        let client = reqwest::Client::new();
        let _ = client.post(url).json(&data).send().await;
    }

    Ok(Json(bot.into()))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_models::v0;
    use rocket::http::{ContentType, Header, Status};
    use validator::Validate;

    #[rocket::async_test]
    async fn create_bot() {
        let harness = TestHarness::new().await;
        let (_, session, _) = harness.new_user().await;

        let response = harness
            .client
            .post("/bots/create")
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(ContentType::JSON)
            .body(
                json!(v0::DataCreateBot {
                    name: TestHarness::rand_string(),
                    bot_type: Some(v0::BotType::PromptBot),
                    model: Some(Default::default())
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);

        let bot: v0::Bot = response.into_json().await.expect("`Bot`");
        assert!(harness.db.fetch_bot(&bot.id).await.is_ok());
    }

    #[test]
    fn test_validate() {
        let bot_data = json!({
            "name":"my_bot",
            "bot_type":"custom_bot",
            "model":{
                "model_name":"gpt-4",
                "prompts":{"system_prompt":""},
                "temperature":2.0
            }
        });

        let mut bot: v0::DataCreateBot = serde_json::from_value(bot_data).unwrap();
        assert!(bot.validate().map_err(|e| println!("{e}")).is_err());

        bot.model.as_mut().unwrap().temperature = 0.5;
        assert!(bot.validate().map_err(|e| println!("{e}")).is_ok());
    }
}
