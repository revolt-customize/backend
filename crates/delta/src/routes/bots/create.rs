use revolt_database::{Bot, BotType, Database, PartialBot, PartialUser, User, UserProfile};
use revolt_models::v0;
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

    let mut bot_information = v0::BotInformation {
        owner_id: user.id.clone(),
        ..Default::default()
    };

    let mut bot_type = BotType::CustomBot;

    if let Some(v0::BotType::PromptBot) = info.bot_type {
        bot_type = BotType::PromptBot;

        match info.bot_information {
            Some(bot_info) => {
                bot_information = bot_info.clone();
                bot_information.owner_id = user.id.clone();

                let bot_model = match bot_info.model {
                    Some(m) => Some(m),
                    None => Some(Default::default()),
                };

                let default_bot_model: v0::BotModel = Default::default();
                bot_information.model = Some(v0::BotModel {
                    model_name: default_bot_model.model_name,
                    ..bot_model.unwrap()
                });
            }
            None => {
                let bot_model = match info.model {
                    Some(m) => Some(m),
                    None => Some(Default::default()),
                };

                let default_bot_model: v0::BotModel = Default::default();
                bot_information.model = Some(v0::BotModel {
                    model_name: default_bot_model.model_name,
                    ..bot_model.unwrap()
                });
            }
        };
    }

    let mut partial_user = PartialUser {
        bot: Some(bot_information.clone().into()),
        ..Default::default()
    };

    if let Some(profile_data) = info.profile {
        partial_user.profile = Some(UserProfile {
            content: profile_data.content.unwrap_or("".into()),
            ..Default::default()
        })
    }

    let (mut bot, bot_user) = Bot::create_with_user(
        db,
        info.name.clone(),
        user.id.clone(),
        PartialBot {
            bot_type: Some(bot_type.clone()),
            ..Default::default()
        },
        partial_user,
    )
    .await?;

    Bot::prepare_default_channel_for_bot(db, &mut bot, &bot_user, &user).await?;

    let _ = create_bot_in_bot_server(&bot, &bot_user, &user).await;

    Ok(Json(bot.into()))
}

async fn create_bot_in_bot_server(bot: &Bot, bot_user: &User, bot_owner: &User) -> Result<()> {
    let config = revolt_config::config().await;

    let bot_type = bot.bot_type.as_ref().unwrap();
    if *bot_type != BotType::PromptBot || config.hosts.promptserv.is_empty() {
        return Ok(());
    }

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

    let host = config.hosts.promptserv;
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
    use revolt_database::{BotInformation, Invite, UserProfile};
    use revolt_models::v0;
    use rocket::http::{ContentType, Header, Status};
    use validator::Validate;

    #[rocket::async_test]
    async fn create_bot() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let response = harness
            .client
            .post("/bots/create")
            .header(Header::new("x-session-token", session.token.to_string()))
            .header(ContentType::JSON)
            .body(
                json!(v0::DataCreateBot {
                    name: TestHarness::rand_string(),
                    bot_type: Some(v0::BotType::PromptBot),
                    model: Some(Default::default()),
                    bot_information: Some(v0::BotInformation {
                        owner_id: "wrong id".into(),
                        model: Some(v0::BotModel {
                            model_name: "model name".into(),
                            prompts: v0::PromptTemplate {
                                system_prompt: "system_prompt".into(),
                                role_requirements: "role_requirements".into(),
                            },
                            temperature: 0.5
                        }),
                        welcome: Some("welcome msg".into())
                    }),
                    profile: Some(v0::UserProfileData {
                        content: Some("background msg".into()),
                        ..Default::default()
                    })
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);

        let bot_response: v0::Bot = response.into_json().await.expect("`Bot`");
        let bot = harness.db.fetch_bot(&bot_response.id).await.unwrap();
        let bot_user = harness.db.fetch_user(&bot_response.id).await.unwrap();

        let mut bot_model: v0::BotModel = Default::default();
        bot_model.prompts.role_requirements = "role_requirements".into();
        bot_model.prompts.system_prompt = "system_prompt".into();
        bot_model.temperature = 0.5;

        // check user.bot field
        assert_eq!(
            Some(BotInformation {
                owner: user.id,
                model: Some(bot_model.into()),
                welcome: Some("welcome msg".into()),
            }),
            bot_user.bot
        );

        // check user.profile field
        assert_eq!(
            Some(UserProfile {
                content: "background msg".into(),
                background: None
            }),
            bot_user.profile
        );

        assert!(bot.server_invite.is_some());
        assert!(bot.default_server.is_some());

        let server = harness
            .db
            .fetch_server(&bot.default_server.unwrap())
            .await
            .unwrap();
        assert_eq!(4, server.channels.len());
        let channels = harness.db.fetch_channels(&server.channels).await.unwrap();
        assert_eq!(4, channels.len());

        let invite = harness
            .db
            .fetch_invite(bot.server_invite.as_ref().unwrap())
            .await
            .unwrap();

        match invite {
            Invite::Server {
                code,
                server: _server,
                channel,
                ..
            } => {
                assert_eq!(Some(code), bot.server_invite);
                assert_eq!(_server, server.id);
                assert_eq!(channel, channels[0].id());
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_validate() {
        let bot_data = json!({
            "name":"my_bot",
            "bot_type":"custom_bot",
            "model":{
                "model_name":"gpt-4",
                "welcome":"hello, welcome",
                "prompts":{"system_prompt":"","role_requirements":""},
                "temperature":2.0
            }
        });

        let mut bot: v0::DataCreateBot = serde_json::from_value(bot_data).unwrap();
        assert!(bot.validate().map_err(|e| println!("{e}")).is_err());

        bot.model.as_mut().unwrap().temperature = 0.5;
        assert!(bot.validate().map_err(|e| println!("{e}")).is_ok());
    }
}
