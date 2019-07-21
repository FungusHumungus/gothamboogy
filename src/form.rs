use core::str;
use gotham::handler::{HandlerError, IntoHandlerError};
use gotham::state::{FromState, State};
use futures::{Future, Stream};
use hyper::{Body, StatusCode};


fn bad_request<E>(e: E) -> HandlerError
where
    E: std::error::Error + Send + 'static,
{
    e.into_handler_error().with_status(StatusCode::BAD_REQUEST)
}


/// Extract the form parameters into a struct using Serde
pub fn extract_form<T>(state: &mut State) -> impl Future<Item = T, Error = HandlerError>
where
    T: serde::de::DeserializeOwned,
{
    Body::take_from(state)
        .concat2()
        .map_err(bad_request)
        .and_then(|body| {
            let b = body.to_vec();
            str::from_utf8(&b)
                .map_err(bad_request)
                .and_then(|s| serde_urlencoded::de::from_str::<T>(s).map_err(bad_request))
        })
}
 
