use std::collections::HashMap;

use crate::util::regex::RE_USERNAME;

use nanoid::nanoid;
use revolt_quark::{
    models::{user::BotInformation, Bot, Channel, Invite, Server, User},
    variables::delta::MAX_BOT_COUNT,
    Db, Error, Result, DEFAULT_PERMISSION_SERVER,
};

use rocket::serde::json::Json;
use serde::Deserialize;
use ulid::Ulid;
use validator::Validate;

/// # Bot Details
#[derive(Validate, Deserialize, JsonSchema)]
pub struct DataCreateBot {
    /// Bot username
    #[validate(length(min = 2, max = 32), regex = "RE_USERNAME")]
    name: String,
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

    let info = info.into_inner();
    info.validate()
        .map_err(|error| Error::FailedValidation { error })?;

    if db.get_number_of_bots_by_user(&user.id).await? >= *MAX_BOT_COUNT {
        return Err(Error::ReachedMaximumBots);
    }

    let id = Ulid::new().to_string();
    let username = User::validate_username(info.name)?;
    let bot_user = User {
        id: id.clone(),
        discriminator: User::find_discriminator(db, &username, None).await?,
        username: username.clone(),
        bot: Some(BotInformation {
            owner: user.id.clone(),
        }),
        ..Default::default()
    };

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

    db.insert_channel(&channel).await?;

    let server = Server {
        id: server_id.clone(),
        owner: user.id.clone(),
        name: username + "的社区",
        description: None,
        channels: vec![channel_id],
        nsfw: false,
        default_permissions: *DEFAULT_PERMISSION_SERVER as i64,
        ..Default::default()
    };
    let invite = Invite::create(db, &user, &channel).await?;
    server.create(db).await?;
    let bot = Bot {
        id,
        owner: user.id.clone(),
        token: nanoid!(64),
        default_server: Some(server_id),
        server_invite: Some(invite.code().to_string()),
        ..Default::default()
    };
    db.insert_user(&bot_user).await?;
    db.insert_bot(&bot).await?;
    server.create_member(db, user, Some(vec![channel])).await?;
    Ok(Json(bot))
}
