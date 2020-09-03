use gotham::{
    handler::HandlerFuture,
    helpers::http::response::create_empty_response,
    middleware::Middleware,
    state::{State, StateData},
};
use hyper::StatusCode;
use redis_async::client::paired::{paired_connect, PairedConnection};
use std::pin::Pin;

pub struct RedisConnection {
    pub conn: PairedConnection,
}

impl StateData for RedisConnection {}

#[derive(Clone, NewMiddleware)]
pub struct RedisMiddleware;

impl Middleware for RedisMiddleware {
    fn call<Chain>(self, mut state: State, chain: Chain) -> Pin<Box<HandlerFuture>>
    where
        Chain: FnOnce(State) -> Pin<Box<HandlerFuture>> + std::marker::Send + 'static,
    {
        Box::pin(async {
            let addr = "127.0.0.1:6379".parse().unwrap();
            match paired_connect(&addr).await {
                Ok(conn) => {
                    state.put(RedisConnection { conn });
                    chain(state).await
                }
                Err(_) => {
                    let res = create_empty_response(&state, StatusCode::INTERNAL_SERVER_ERROR);
                    Ok((state, res))
                }
            }
        })
    }
}
