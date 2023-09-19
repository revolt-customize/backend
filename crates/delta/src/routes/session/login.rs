use revolt_database::User;
use revolt_quark::authifier::Authifier;
use revolt_result::Result;
use rocket::serde::json::Json;
use rocket::State;
use rocket_authifier::routes::session::login::ResponseLogin;
use serde::{Deserialize, Serialize};

/// # Login Data
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct DataLogin {
    /// Friendly name used for the session
    pub friendly_name: Option<String>,
}

/// # Login
///
/// Login to an account.
#[openapi(tag = "Session")]
#[post("/login", data = "<data>")]
pub async fn login(
    user: User,
    // db: &State<Database>,
    authifier: &State<Authifier>,
    data: Json<DataLogin>,
    // cookies: &CookieJar<'_>,
    // headers: Headers<'_>,
) -> Result<Json<ResponseLogin>> {
    let data = data.into_inner();
    let account = authifier.database.find_account(&user.id).await.unwrap();
    let session = account
        .create_session(authifier, data.friendly_name.unwrap_or("".into()))
        .await
        .unwrap();

    Ok(Json(ResponseLogin::Success(session)))
}
