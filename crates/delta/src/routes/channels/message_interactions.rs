use revolt_quark::{
    events::client::EventV1,
    models::{message::Interaction, User},
    perms, Db, Error, Permission, Ref, Result,
};

use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Interaction
#[derive(Validate, Serialize, Deserialize, JsonSchema)]
pub struct DataInteraction {
    /// Interaction content
    #[validate(length(min = 0, max = 2000))]
    content: String,
}

/// # Interactions
///
/// Interactions with the Bot.
#[openapi(tag = "Messaging")]
#[post("/<target>/messages/<msg>/interactions", data = "<interaction>")]
pub async fn req(
    db: &Db,
    user: User,
    target: Ref,
    msg: Ref,
    interaction: Json<DataInteraction>,
) -> Result<Json<Interaction>> {
    let interaction = interaction.into_inner();
    interaction
        .validate()
        .map_err(|error| Error::FailedValidation { error })?;

    // Ensure we have permissions to send a message
    let channel = target.as_channel(db).await?;

    let mut permissions = perms(&user).channel(&channel);
    permissions
        .throw_permission_and_view_channel(db, Permission::SendMessage)
        .await?;

    let message = msg.as_message(db).await?;

    let interaction_message = Interaction {
        message_id: message.id.clone(),
        nonce: message.nonce.unwrap_or("".into()),
        channel_id: channel.id().to_string(),
        author_id: user.id.clone(),
        content: interaction.content,
    };

    EventV1::Interaction(interaction_message.clone())
        .p(channel.id().to_string())
        .await;

    Ok(Json(interaction_message))
}
