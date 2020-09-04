#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate redis_async;
#[macro_use]
extern crate log;

use gotham::{
    handler::assets::FileOptions,
    middleware::session::NewSessionMiddleware,
    pipeline::{
        new_pipeline,
        set::{finalize_pipeline_set, new_pipeline_set},
    },
    router::builder::*,
    router::Router,
};

mod auth;
mod database;
mod form;
mod handlers;
mod redis;


fn router() -> Router {
    let middleware = NewSessionMiddleware::new(redis::RedisBackend)
        .with_session_type::<auth::Session>()
        .insecure();

    let pipelines = new_pipeline_set();
    let (pipelines, default) = pipelines.add(
        new_pipeline()
            .add(redis::RedisMiddleware)
            .add(middleware)
            .build(),
    );
    let (pipelines, secured) = pipelines.add(new_pipeline().add(auth::AuthMiddleware).build());

    let pipeline_set = finalize_pipeline_set(pipelines);

    let default_chain = (default, ());
    let secured_chain = (secured, default_chain);

    build_router(default_chain, pipeline_set, |route| {
        // Setup the root route with the secured middleware.
        route.with_pipeline_chain(secured_chain, |route| {
            route.get("/").to(handlers::index);
        });

        // The other routes are not secured.
        route.get("/login").to(handlers::login_get);
        route.post("/login").to(handlers::login_post);

        route.get("/register").to(handlers::register_get);
        route.post("/register").to(handlers::register_post);

        route.get("assets/*").to_dir(
            FileOptions::new("assets/")
                .with_cache_control("no-cache")
                .with_gzip(true)
                .build(),
        )
    })
}

fn main() {
    env_logger::init();

    let addr = "127.0.0.1:6767";
    trace!("Listening on http://{}", addr);

    gotham::start(addr, router())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;
    use hyper::{header, StatusCode};
    use mime;

    #[test]
    fn visit_root_returns_unauthorized() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn visit_root_after_login_works() {
        let test_server = TestServer::new(router()).unwrap();

        test_server
            .client()
            .post(
                "http://localhost/register",
                "username=onk&password=ponk",
                mime::MULTIPART_FORM_DATA,
            )
            .perform()
            .unwrap();

        let response = test_server
            .client()
            .post(
                "http://localhost/login",
                "username=onk&password=ponk",
                mime::MULTIPART_FORM_DATA,
            )
            .perform()
            .unwrap();

        let headers = response.headers().clone();
        let cookie = headers.get(header::SET_COOKIE).unwrap();

        let response = test_server
            .client()
            .get("http://localhost/")
            .with_header(header::COOKIE, cookie.to_owned())
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
