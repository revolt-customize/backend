use crate::util::regex::RE_USERNAME;
use revolt_quark::{
    authifier::models::Session,
    models::{Channel, User},
    Database, EmptyResponse, Error, Result,
};

use rocket::{serde::json::Json, State};
use serde::{Deserialize, Serialize};
use ulid::Ulid;
use validator::Validate;

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
) -> Result<EmptyResponse> {
    if user.is_some() {
        return Err(Error::AlreadyOnboarded);
    }

    let data = data.into_inner();
    data.validate()
        .map_err(|error| Error::FailedValidation { error })?;

    let username = User::validate_username(data.username)?;
    let user = User {
        id: session.user_id,
        discriminator: User::find_discriminator(db, &username, None).await?,
        username,
        ..Default::default()
    };
    let res = db.insert_user(&user).await.map(|_| EmptyResponse);
    if res.is_ok() {
        let create_user = user.clone();
        let server: revolt_quark::models::Server =
            db.fetch_server("01H7A2D436ZP77QPQ0XK7HBG1H").await?;
        server.create_member(db, create_user, None).await?;
        let group_id = Ulid::new().to_string();
        let group = Channel::Group {
            id: group_id.clone(),
            name: String::from("多模型群聊"),
            owner: user.id.clone(),
            description: Some(String::from("默认群聊，可以通过@来调用大模型")),
            recipients: vec![user.id.clone()],
            icon: None,
            last_message_id: None,
            permissions: None,
            nsfw: false,
        };
        group.create_group(db).await?;
    };
    res
}
