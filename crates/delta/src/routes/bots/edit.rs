use revolt_database::{util::reference::Reference, Database, PartialBot, User};
use revolt_models::v0::{self, DataEditBot};
use revolt_result::{create_error, Result};
use rocket::State;

use rocket::serde::json::Json;
use validator::Validate;

/// # Edit Bot
///
/// Edit bot details by its id.
#[openapi(tag = "Bots")]
#[patch("/<target>", data = "<data>")]
pub async fn edit_bot(
    db: &State<Database>,
    user: User,
    target: Reference,
    data: Json<DataEditBot>,
) -> Result<Json<v0::Bot>> {
    let data = data.into_inner();
    data.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let mut bot = target.as_bot(db).await?;
    if bot.owner != user.id {
        return Err(create_error!(NotFound));
    }

    if let Some(name) = data.name {
        let mut user = db.fetch_user(&bot.id).await?;
        user.update_username(db, name).await?;
    }

    if data.public.is_none()
        && data.analytics.is_none()
        && data.interactions_url.is_none()
        && data.remove.is_none()
    {
        return Ok(Json(bot.into()));
    }

    if let Some(true) = data.public {
        let bots = db.fetch_discoverable_bots().await?;
        let mut user_ids = bots
            .into_iter()
            .map(|x| x.id.clone())
            .collect::<Vec<String>>();

        user_ids.push(bot.id.clone());
        let users = db.fetch_users(&user_ids).await?;
        let bot_user = users
            .iter()
            .find(|x| *x.id == bot.id)
            .ok_or_else(|| create_error!(NotFound))?;

        if users
            .iter()
            .any(|x| *x.id != bot_user.id && *x.username == bot_user.username)
        {
            return Err(create_error!(DuplicatePublicBotName));
        }
    }

    let DataEditBot {
        public,
        analytics,
        interactions_url,
        remove,
        ..
    } = data;

    let partial = PartialBot {
        public,
        analytics,
        interactions_url,
        ..Default::default()
    };

    bot.update(
        db,
        partial,
        remove
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.into())
            .collect(),
    )
    .await?;

    Ok(Json(bot.into()))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{Bot, PartialBot};
    use revolt_models::v0::{self, FieldsBot};
    use revolt_result::{Error, ErrorType};
    use rocket::http::{ContentType, Header, Status};

    #[rocket::async_test]
    async fn edit_bot() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let bot = Bot::create(&harness.db, TestHarness::rand_string(), &user, None)
            .await
            .expect("`Bot`");

        let response = harness
            .client
            .patch(format!("/bots/{}", bot.id))
            .header(ContentType::JSON)
            .body(
                json!(v0::DataEditBot {
                    public: Some(true),
                    remove: Some(vec![FieldsBot::Token]),
                    ..Default::default()
                })
                .to_string(),
            )
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        // info!("{}", response.into_string().await.unwrap());
        assert_eq!(response.status(), Status::Ok);

        let updated_bot: v0::Bot = response.into_json().await.expect("`Bot`");
        assert!(!bot.public);
        assert!(updated_bot.public);
    }

    #[rocket::async_test]
    async fn add_duplicated_public_bot() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let bot_name = TestHarness::rand_string();
        let bot1 = Bot::create(&harness.db, bot_name.clone(), &user, None)
            .await
            .expect("`Bot`");

        let _ = Bot::create(
            &harness.db,
            bot_name.clone(),
            &user,
            PartialBot {
                public: Some(true),
                ..Default::default()
            },
        )
        .await
        .expect("`Bot`");

        let response = harness
            .client
            .patch(format!("/bots/{}", bot1.id))
            .header(ContentType::JSON)
            .body(
                json!(v0::DataEditBot {
                    public: Some(true),
                    ..Default::default()
                })
                .to_string(),
            )
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);

        let err = response.into_json::<Error>().await.unwrap();
        assert_eq!(err.error_type, ErrorType::DuplicatePublicBotName);
    }
}
