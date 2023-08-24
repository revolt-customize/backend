use revolt_quark::{
    events::client::EventV1,
    models::message::PartialMessage,
    models::{Message, User},
    perms, Db, Error, Permission, Ref, Result,
};

use rocket::tokio::io::AsyncReadExt;
use rocket::{
    data::{Data, ToByteUnit},
    serde::json::Json,
};

/// Streaming Message
///
/// Patch a streaming message that you've previously sent.
#[openapi(tag = "Messaging")]
#[post("/<target>/messages/<msg>/stream", data = "<data>")]
pub async fn req(
    db: &Db,
    user: User,
    target: Ref,
    msg: Ref,
    data: Data<'_>,
) -> Result<Json<Message>> {
    // Ensure we have permissions to send a message
    let channel = target.as_channel(db).await?;
    let mut permissions = perms(&user).channel(&channel);
    permissions
        .throw_permission_and_view_channel(db, Permission::SendMessage)
        .await?;

    let mut message = msg.as_message(db).await?;
    if message.channel != channel.id() {
        return Err(Error::NotFound);
    }

    if message.author != user.id {
        return Err(Error::CannotEditMessage);
    }

    let mut reader = data.open(512.kibibytes());
    let mut buffer = bytes::BytesMut::with_capacity(1024);
    let mut result = String::new();

    if let Some(ref v) = message.content {
        result.push_str(v);
    }

    loop {
        let n = reader.read_buf(&mut buffer).await.map_err(|error| {
            warn!("read error {error}");
            Error::InternalError
        })?;

        if n == 0 {
            EventV1::MessagePatch {
                message_id: msg.id.clone(),
                content: "".into(),
                is_end: true,
            }
            .p(channel.id().to_string())
            .await;
            break;
        }

        let message_patch = String::from_utf8_lossy(&buffer[..n]).to_string();
        result.push_str(&message_patch);

        EventV1::MessagePatch {
            message_id: message.id.clone(),
            content: message_patch,
            is_end: false,
        }
        .p(channel.id().to_string())
        .await;

        buffer.clear();
    }

    // Message::validate_sum(&Some(result.to_string()), Default::default())?;

    let mut partial = PartialMessage {
        ..Default::default()
    };

    partial.content = Some(result.to_string());
    message.apply_options(partial.clone());
    db.update_message(&message.id, &partial).await?;

    Ok(Json(message.to_owned()))
}
