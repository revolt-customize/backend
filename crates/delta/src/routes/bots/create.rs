use crate::util::regex::RE_USERNAME;

use nanoid::nanoid;
use revolt_quark::{
    models::{bot::BotType, prompt::BotModel, user::BotInformation, Bot, User},
    variables::delta::MAX_BOT_COUNT,
    Db, Error, Result,
};

use rocket::serde::json::Json;
use serde::Deserialize;
use ulid::Ulid;
use validator::Validate;

/// # Bot Details
#[derive(Validate, Deserialize, JsonSchema, Debug)]
pub struct DataCreateBot {
    /// Bot username
    #[validate(length(min = 2, max = 32), regex = "RE_USERNAME")]
    name: String,
    bot_type: Option<BotType>,
    #[validate]
    model: Option<BotModel>,
}

/// # Create Bot
///
/// Create a new Revolt bot.
#[openapi(tag = "Bots")]
#[post("/create", data = "<info>")]
pub async fn create_bot(db: &Db, user: User, info: Json<DataCreateBot>) -> Result<Json<Bot>> {
    if user.bot.is_some() {
        return Err(Error::IsBot);
    }

    let mut info = info.into_inner();
    info.validate()
        .map_err(|error| Error::FailedValidation { error })?;

    if db.get_number_of_bots_by_user(&user.id).await? >= *MAX_BOT_COUNT {
        return Err(Error::ReachedMaximumBots);
    }

    let mut bot_information = BotInformation {
        owner: user.id.clone(),
        model: None,
    };

    match info.bot_type {
        Some(BotType::CustomBot) => (),
        Some(BotType::PromptBot) => match info.model {
            Some(m) => bot_information.model = Some(m),
            None => {
                bot_information.model = Some(Default::default());
            }
        },
        None => info.bot_type = Some(BotType::CustomBot),
    }

    let id = Ulid::new().to_string();
    let username = User::validate_username(info.name)?;
    let bot_user = User {
        id: id.clone(),
        discriminator: User::find_discriminator(db, &username, None).await?,
        username,
        bot: Some(bot_information),
        ..Default::default()
    };

    let bot = Bot {
        id,
        owner: user.id,
        token: nanoid!(64),
        bot_type: info.bot_type,
        ..Default::default()
    };

    db.insert_user(&bot_user).await?;
    db.insert_bot(&bot).await?;
    Ok(Json(bot))
}

#[cfg(test)]
mod tests {
    use validator::Validate;

    use crate::routes::bots::create::DataCreateBot;

    #[test]
    fn test_validate() {
        let bot_data = json!({
            "name":"my_bot",
            "bot_type":"custom_bot",
            "model":{
                "model_name":"gpt-4",
                "prompts":{"system_prompt":""},
                "temperature":1.0
            }
        });

        let bot: DataCreateBot = serde_json::from_value(bot_data).unwrap();
        assert!(bot.validate().map_err(|e| println!("{e}")).is_ok());
    }
}
