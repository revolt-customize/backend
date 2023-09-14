use revolt_database::{Bot, BotType, Channel, Database, Invite, Member, PartialBot, Server, User};
use revolt_models::v0;
use revolt_permissions::DEFAULT_PERMISSION_SERVER;
use revolt_result::{create_error, Result};
use rocket::serde::json::Json;
use rocket::State;
use std::collections::HashMap;
use ulid::Ulid;
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

    let (server, channel) = create_default_channel_for_bot(db, info.name.clone(), &owner).await?;

    let mut invite_code: Option<String> = None;
    let mut default_server: Option<String> = None;

    if let Invite::Server { code, server, .. } =
        Invite::create_channel_invite(db, user.id.clone(), &channel).await?
    {
        invite_code = Some(code);
        default_server = Some(server)
    }

    let bot = Bot::create(
        db,
        info.name.clone(),
        &owner,
        PartialBot {
            bot_type: Some(bot_type.clone()),
            server_invite: invite_code,
            default_server,
            ..Default::default()
        },
    )
    .await?;

    let bot_user = db.fetch_user(&bot.id).await?;

    Member::create(db, &server, &user).await?;
    Member::create(db, &server, &bot_user).await?;

    let config = revolt_config::config().await;

    if bot_type == BotType::PromptBot && !config.api.botservice.bot_server.is_empty() {
        let _ = create_bot_at_bot_server(&bot, &bot_user, &user).await;
    }

    Ok(Json(bot.into()))
}

async fn create_default_channel_for_bot(
    db: &Database,
    bot_name: String,
    user: &User,
) -> Result<(Server, Channel)> {
    let channel_id = Ulid::new().to_string();
    let server_id = Ulid::new().to_string();

    let channel = Channel::TextChannel {
        id: channel_id.clone(),
        server: server_id.clone(),
        name: "默认频道".into(),
        description: None,
        icon: None,
        last_message_id: None,
        default_permissions: None,
        role_permissions: HashMap::new(),
        nsfw: false,
    };

    channel.create(db).await?;

    let server = Server {
        id: server_id.clone(),
        owner: user.id.clone(),
        name: bot_name + "的主页",
        description: None,
        channels: vec![channel_id],
        nsfw: false,
        default_permissions: *DEFAULT_PERMISSION_SERVER as i64,
        ..Default::default()
    };

    server.create(db).await?;
    Ok((server, channel))
}

async fn create_bot_at_bot_server(bot: &Bot, bot_user: &User, bot_owner: &User) -> Result<()> {
    let model = bot_user.bot.as_ref().unwrap().model.as_ref().unwrap();

    let data = CreatePromptBotReq {
        user_id: bot_owner.id.clone(),
        user_name: bot_owner.username.clone(),
        bot_id: bot.id.clone(),
        bot_name: bot_user.username.clone(),
        bot_token: bot.token.clone(),
        model_name: model.model_name.clone(),
        prompt_template: model.prompts.system_prompt.clone(),
        temperature: model.temperature,
    };

    let config = revolt_config::config().await;
    let host = config.api.botservice.bot_server;
    let url = format!("{host}/api/rest/v1/bot/create");
    let client = reqwest::Client::new();
    let response = client
        .post(url.clone())
        .json(&data)
        .send()
        .await
        .map_err(|_| create_error!(InternalError))?
        .text()
        .await
        .map_err(|_| create_error!(InternalError))?;

    let data_json = serde_json::to_string(&data).map_err(|_| create_error!(InternalError))?;
    info!("bot-server:\nurl:{url}\ndata:{data_json}\nresponse:{response}");
    Ok(())
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

        let bot_response: v0::Bot = response.into_json().await.expect("`Bot`");
        let bot = harness.db.fetch_bot(&bot_response.id).await.unwrap();

        assert!(bot.server_invite.is_some());
        assert!(bot.default_server.is_some());
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
