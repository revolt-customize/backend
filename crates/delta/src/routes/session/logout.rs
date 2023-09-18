use crate::util::header::Headers;
use revolt_models::v0;
use revolt_quark::authifier::{models::Session, Authifier};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Logout
///
/// Delete current session.
#[openapi(tag = "Session")]
#[post("/logout")]
pub async fn logout(
    authifier: &State<Authifier>,
    session: Session,
    headers: Headers<'_>,
) -> Result<EmptyResponse> {
    let config = revolt_config::config().await;
    let service_url = headers.0.get("Referer").next().unwrap_or("");
    let url = format!(
        "{}/v1/logout?service={}",
        config.api.botservice.chatall_server, service_url
    );

    let client = reqwest::Client::new();
    let _: v0::UUAPResponse = client
        .get(url.clone())
        .send()
        .await
        .expect("SendRequestFailed")
        .json()
        .await
        .expect("ParseJsonFailed");

    session
        .delete(authifier)
        .await
        .map_err(|_| create_error!(InternalError))?;

    Ok(EmptyResponse)
}
