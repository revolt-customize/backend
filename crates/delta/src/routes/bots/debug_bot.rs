use revolt_result::Result;
use rocket::serde::json::Json;
use rocket_empty::EmptyResponse;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct RequestBody {
    pub user_name: String,
    pub prompt_template: String,
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// # Debug a prompt bot
///
/// Debug API for prompt bots
#[openapi(tag = "Bots")]
#[post("/debug-chat", data = "<data>")]
pub async fn req(_data: Json<RequestBody>) -> Result<EmptyResponse> {
    Ok(EmptyResponse)
}
