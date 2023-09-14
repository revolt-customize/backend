use revolt_quark::authifier::{models::Session, Authifier};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Logout
///
/// Delete current session.
#[openapi(tag = "Session")]
#[post("/logout")]
pub async fn logout(authifier: &State<Authifier>, session: Session) -> Result<EmptyResponse> {
    session
        .delete(authifier)
        .await
        .map_err(|_| create_error!(InternalError))?;

    Ok(EmptyResponse)
}
