use gotham::{
    handler::HandlerFuture,
    helpers::http::response::create_empty_response,
    middleware::{
        session::{Backend, NewBackend, SessionError, SessionIdentifier},
        Middleware, NewMiddleware,
    },
    state::{FromState, State, StateData},
};
use hyper::StatusCode;
use redis_async::client::paired::{paired_connect, PairedConnection};
use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
pub struct RedisConnection {
    pub conn: PairedConnection,
}

impl StateData for RedisConnection {}

#[derive(Clone)]
pub struct RedisMiddleware;

impl NewMiddleware for RedisMiddleware {
    type Instance = RedisMiddleware;

    fn new_middleware(&self) -> anyhow::Result<Self::Instance> {
        Ok(RedisMiddleware)
    }
}

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

type SessionFuture = dyn Future<Output = Result<Option<Vec<u8>>, SessionError>> + Send;

#[derive(Clone)]
pub struct RedisBackend;

impl NewBackend for RedisBackend {
    type Instance = RedisBackend;

    fn new_backend(&self) -> anyhow::Result<Self::Instance> {
        Ok(RedisBackend)
    }
}

impl Backend for RedisBackend {
    fn persist_session(
        &self,
        state: &State,
        identifier: SessionIdentifier,
        content: &[u8],
    ) -> Result<(), SessionError> {
        info!("Persisting session {}", identifier.value);

        let conn = RedisConnection::borrow_from(&state);
        conn.conn.send_and_forget(resp_array!["HSET", "session", identifier.value, content]);
        Ok(())
    }

    fn read_session(
        &self,
        state: &State,
        identifier: SessionIdentifier,
    ) -> Pin<Box<SessionFuture>> {
        info!("Reading session {}", identifier.value);

        let conn = RedisConnection::borrow_from(&state);
        let conn = conn.clone();
        Box::pin(async move {
            let result = conn.conn
                .send(resp_array!["HGET", "session", identifier.value])
                .await
                .ok();
            Ok(result)
        })
    }

    fn drop_session(&self, _identifier: SessionIdentifier) -> Result<(), SessionError> {
        unimplemented!()
    }
}
