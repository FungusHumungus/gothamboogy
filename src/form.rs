use core::str;
use gotham::handler::{HandlerError, IntoHandlerError};
use gotham::hyper::{body, Body, StatusCode};
use gotham::state::{FromState, State};

fn bad_request<E>(e: E) -> HandlerError
where
    E: std::error::Error + Send + 'static,
{
    e.into_handler_error().with_status(StatusCode::BAD_REQUEST)
}

/// Extract the form parameters into a struct using Serde
pub async fn extract_form<T>(state: &mut State) -> Result<T, HandlerError>
where
    T: serde::de::DeserializeOwned,
{
    let body = body::to_bytes(Body::take_from(state))
        .await
        .map_err(bad_request)?
        .to_vec();
    let s = str::from_utf8(&body).unwrap();
    serde_urlencoded::de::from_str::<T>(s).map_err(bad_request)
}
