use futures::future::join_all;
use revolt_database::Database;
use revolt_models::v0::OwnedBotsResponse;
use revolt_result::Result;
use rocket::serde::json::Json;
use rocket::State;

/// # Fetch discoverable Bots
///
/// Fetch all of the bots that discoverable.
#[openapi(tag = "Bots")]
#[get("/discover")]
pub async fn fetch_discoverable_bots(db: &State<Database>) -> Result<Json<OwnedBotsResponse>> {
    let mut bots = db.fetch_discoverable_bots().await?;
    let user_ids = bots
        .iter()
        .map(|x| x.id.to_owned())
        .collect::<Vec<String>>();

    let mut users = db.fetch_users(&user_ids).await?;

    // Ensure the lists match up exactly.
    bots.sort_by(|a, b| a.id.cmp(&b.id));
    users.sort_by(|a, b| a.id.cmp(&b.id));

    // Ok(Json(OwnedBotsResponse { users, bots }))
    Ok(Json(OwnedBotsResponse {
        users: join_all(users.into_iter().map(|user| user.into_self())).await,
        bots: bots.into_iter().map(|bot| bot.into()).collect(),
    }))
}
