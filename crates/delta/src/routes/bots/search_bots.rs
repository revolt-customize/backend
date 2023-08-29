use futures::future::join_all;
use revolt_database::Database;
use revolt_models::v0;
use revolt_result::Result;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};

/// # Query Parameters
#[derive(Serialize, Deserialize, JsonSchema, FromForm, Debug)]
pub struct ParamSearchBot {
    bot_type: Option<String>,
}

/// # Search Bots
///
/// Search Bots by given conditions.
#[openapi(tag = "Bots")]
#[get("/search?<options..>")]
pub async fn req(
    db: &State<Database>,
    options: ParamSearchBot,
) -> Result<Json<v0::OwnedBotsResponse>> {
    if let Some(bot_type) = options.bot_type {
        let mut bots = db.search_bots_by_type(bot_type.as_str()).await?;
        let user_ids = bots
            .iter()
            .map(|x| x.id.to_owned())
            .collect::<Vec<String>>();

        let mut users = db.fetch_users(&user_ids).await?;

        // Ensure the lists match up exactly.
        bots.sort_by(|a, b| a.id.cmp(&b.id));
        users.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(Json(v0::OwnedBotsResponse {
            users: join_all(users.into_iter().map(|user| user.into_self())).await,
            bots: bots.into_iter().map(|bot| bot.into()).collect(),
        }))
    } else {
        Ok(Json(v0::OwnedBotsResponse {
            users: Default::default(),
            bots: Default::default(),
        }))
    }
}
