use revolt_quark::{
    events::client::EventV1,
    models::message::PartialMessage,
    models::{message::DataMessageSend, Message, User},
    perms,
    types::push::MessageAuthor,
    web::idempotency::IdempotencyKey,
    Db, Error, Permission, Ref, Result,
};

use rocket::data::{Data, ToByteUnit};
use rocket::serde::json::Json;
use rocket::tokio::io::AsyncReadExt;
use validator::Validate;

/// # Send Streaming Message
///
/// Sends a streaming message to the given channel.
#[openapi(tag = "Messaging")]
#[post("/<target>/stream_messages", data = "<data>")]
pub async fn req(
    db: &Db,
    user: User,
    target: Ref,
    // data: Json<DataMessageSend>,
    data: Data<'_>,
    idempotency: IdempotencyKey,
) -> Result<Json<Message>> {
    let mut reader = data.open(512.kibibytes());
    let mut buffer = bytes::BytesMut::with_capacity(1024);

    let mut message: Option<Message> = None;
    let mut result = String::new();

    loop {
        let n = reader.read_buf(&mut buffer).await.map_err(|error| {
            warn!("{error}");
            Error::InternalError
        })?;
        if n == 0 {
            let unwrapped_message = message.as_ref().expect("message should be initialized");
            EventV1::MessagePatch {
                message_id: unwrapped_message.id.clone(),
                content: "".into(),
                is_end: true,
            }
            .p(unwrapped_message.channel.clone())
            .await;
            break;
        }

        match message {
            None => {
                let data_message_send: DataMessageSend =
                    serde_json::from_slice(&buffer[..n]).map_err(|_| Error::InvalidProperty)?;

                if let Some(ref v) = data_message_send.content {
                    result.push_str(&v.clone());
                }

                message = Some(
                    generate_message(&user, &target, db, data_message_send, idempotency.clone())
                        .await?,
                );
            }

            Some(ref mes) => {
                let message_patch = String::from_utf8_lossy(&buffer[..n]).to_string();
                result.push_str(&message_patch);

                EventV1::MessagePatch {
                    message_id: mes.id.clone(),
                    content: message_patch,
                    is_end: false,
                }
                .p(mes.channel.clone())
                .await;
            }
        }

        buffer.clear();
    }

    let unwrapped_message = message.as_mut().expect("message should be initialized");

    let mut partial = PartialMessage {
        ..Default::default()
    };

    partial.content = Some(result.to_string());

    unwrapped_message.apply_options(partial.clone());
    db.update_message(&unwrapped_message.id, &partial).await?;

    Ok(Json(unwrapped_message.to_owned()))
}

async fn generate_message(
    user: &User,
    target: &Ref,
    db: &Db,
    data: DataMessageSend,
    idempotency: IdempotencyKey,
) -> Result<Message> {
    data.validate()
        .map_err(|error| Error::FailedValidation { error })?;

    // Ensure we have permissions to send a message
    let channel = target.as_channel(db).await?;

    let mut permissions = perms(user).channel(&channel);
    permissions
        .throw_permission_and_view_channel(db, Permission::SendMessage)
        .await?;

    // Verify permissions for masquerade
    if let Some(masq) = &data.masquerade {
        permissions
            .throw_permission(db, Permission::Masquerade)
            .await?;

        if masq.colour.is_some() {
            permissions
                .throw_permission(db, Permission::ManageRole)
                .await?;
        }
    }

    // Check permissions for embeds
    if data.embeds.as_ref().is_some_and(|v| !v.is_empty()) {
        permissions
            .throw_permission(db, Permission::SendEmbeds)
            .await?;
    }

    // Check permissions for files
    if data.attachments.as_ref().is_some_and(|v| !v.is_empty()) {
        permissions
            .throw_permission(db, Permission::UploadFiles)
            .await?;
    }

    // Ensure interactions information is correct
    if let Some(interactions) = &data.interactions {
        interactions.validate(db, &mut permissions).await?;
    }

    // Create the message
    let message = channel
        .send_message(
            db,
            data,
            MessageAuthor::User(user),
            idempotency,
            permissions
                .has_permission(db, Permission::SendEmbeds)
                .await?,
            Some(true),
        )
        .await?;

    Ok(message)
}
