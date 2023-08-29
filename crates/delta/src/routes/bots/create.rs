use revolt_database::{Bot, Database, User};
use revolt_models::v0;
use revolt_result::{create_error, Result};
use rocket::serde::json::Json;
use rocket::State;
use ulid::Ulid;
use validator::Validate;

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

    let mut info = info.into_inner();
    info.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let max_bot_count = 10;

    if db.get_number_of_bots_by_user(&user.id).await? >= max_bot_count {
        return Err(create_error!(ReachedMaximumBots));
    }

    let mut bot_information = v0::BotInformation {
        owner_id: user.id.clone(),
        model: None,
    };

    match info.bot_type {
        Some(v0::BotType::CustomBot) => (),
        Some(v0::BotType::PromptBot) => match info.model {
            Some(m) => bot_information.model = Some(m),
            None => {
                bot_information.model = Some(Default::default());
            }
        },
        None => info.bot_type = Some(v0::BotType::CustomBot),
    }

    let id = Ulid::new().to_string();
    let username = User::validate_username(info.name)?;
    let bot_user = User {
        id: id.clone(),
        discriminator: User::find_discriminator(db, &username, None).await?,
        username,
        bot: Some(bot_information.into()),
        ..Default::default()
    };

    let bot = Bot {
        id,
        owner: user.id,
        token: nanoid::nanoid!(64),
        bot_type: info.bot_type.map(|x| x.into()),
        ..Default::default()
    };

    db.insert_user(&bot_user).await?;
    db.insert_bot(&bot).await?;
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
