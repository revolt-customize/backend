use revolt_quark::{models::User, Db, Error, Result};
use rocket::serde::json::Json;

/// # Fetch discoverable Bots
///
/// Fetch all of the bots that discoverable.
#[openapi(tag = "Bots")]
#[get("/discover")]
pub async fn fetch_discoverable_bots(db: &Db, user: User) -> Result<Json<Vec<User>>> {
    if user.bot.is_some() {
        return Err(Error::IsBot);
    }

    let bots = db.fetch_discoverable_bots().await?;
    let user_ids = bots
        .iter()
        .map(|x| x.id.to_owned())
        .collect::<Vec<String>>();

    let mut users = db.fetch_users(&user_ids).await?;

    // Ensure the lists match up exactly.
    users.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(Json(users))
}
