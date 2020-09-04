use core::str;
use gotham::handler::{HandlerError};
use gotham::hyper::{body, Body};
use gotham::state::{FromState, State};

fn bad_request<E>(e: E) -> HandlerError
where
    E: Into<anyhow::Error> + Send + 'static,
{
    e.into().into()
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
