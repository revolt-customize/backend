use revolt_rocket_okapi::gen::OpenApiGenerator;
use revolt_rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};
use rocket::{
    http::HeaderMap,
    request::{self, FromRequest},
    Request,
};

pub struct Headers<'a>(pub &'a HeaderMap<'a>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Headers<'r> {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request::Outcome::Success(Headers(request.headers()))
    }
}

impl<'r> OpenApiFromRequest<'r> for Headers<'r> {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> revolt_rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::None)
    }
}
