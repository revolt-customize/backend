use super::fetch_owned::OwnedBotsResponse;
use revolt_quark::{models::bot::BotType, Db, Result};
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

/// # Query Parameters
#[derive(Serialize, Deserialize, JsonSchema, FromForm, Debug)]
pub struct ParamSearchBot {
    bot_type: Option<BotType>,
}

/// # Search Bots
///
/// Search Bots by given conditions.
#[openapi(tag = "Bots")]
#[get("/search?<options..>")]
pub async fn req(db: &Db, options: ParamSearchBot) -> Result<Json<OwnedBotsResponse>> {
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

        Ok(Json(OwnedBotsResponse { users, bots }))
    } else {
        Ok(Json(OwnedBotsResponse {
            users: Default::default(),
            bots: Default::default(),
        }))
    }
}
