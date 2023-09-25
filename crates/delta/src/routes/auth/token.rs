use revolt_database::Database;
use revolt_models::v0;
use revolt_result::{create_error, Result};
use rocket::serde::json::Json;
use rocket::State;

use crate::util::header::Headers;

#[openapi(tag = "Auth")]
#[post("/token")]
pub async fn token(headers: Headers<'_>, db: &State<Database>) -> Result<Json<v0::User>> {
    let auth_token = headers.0.get("Authorization").next().unwrap_or("");
    let user = db
        .fetch_user_by_token(auth_token)
        .await
        .map_err(|_| create_error!(InvalidCredentials))?;

    Ok(Json(user.into_self().await))
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_models::v0;
    use rocket::http::{Header, Status};

    #[rocket::async_test]
    async fn test_auth_token() {
        let harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let response = harness
            .client
            .post("/auth/token")
            .header(Header::new("Authorization", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);

        let user_response: v0::User = response.into_json().await.expect("`User`");
        assert_eq!(user.clone().into_self().await, user_response);

        let user_fetched = harness.db.fetch_user(&user_response.id).await.unwrap();
        assert_eq!(user.into_self().await, user_fetched.into_self().await);
    }
}
