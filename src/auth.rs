use crate::database;
use futures::future;
use gotham::handler::HandlerFuture;
use gotham::helpers::http::response::{create_empty_response, create_response};
use gotham::middleware::session::SessionData;
use gotham::middleware::Middleware;
use gotham::state::{FromState, State};
use hyper::StatusCode;
use std::{collections::HashMap, pin::Pin};

lazy_static! {
    static ref NO_PERMISSION: mustache::Template =
        mustache::compile_path("templates/nopermission.tpl").unwrap();
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Session {
    pub userid: Option<database::User>,
}

/// Middleware to handle authentication and ensure only authenticated users are
/// permitted to the route.
#[derive(Clone, NewMiddleware)]
pub struct AuthMiddleware;

impl Middleware for AuthMiddleware {
    fn call<Chain>(self, state: State, chain: Chain) -> Pin<Box<HandlerFuture>>
    where
        Chain: FnOnce(State) -> Pin<Box<HandlerFuture>> + 'static,
    {
        let session: &Session = SessionData::<Session>::borrow_from(&state);
        match session.userid {
            Some(_) => chain(state),
            None => {
                info!("Authentication request denied");

                let data: HashMap<String, String> = HashMap::new();
                let res = match NO_PERMISSION.render_to_string(&data) {
                    Ok(content) => create_response(
                        &state,
                        StatusCode::UNAUTHORIZED,
                        mime::TEXT_HTML_UTF_8,
                        content.into_bytes(),
                    ),
                    Err(_) => create_empty_response(&state, StatusCode::INTERNAL_SERVER_ERROR),
                };

                Box::pin(future::ok((state, res)))
            }
        }
    }
}
