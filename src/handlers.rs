use crate::{auth, database, form::extract_form, redis::RedisConnection};
use gotham::handler::HandlerFuture;
use gotham::helpers::http::response::{create_empty_response, create_response};
use gotham::middleware::session::SessionData;
use gotham::state::{FromState, State};
use hyper::{header, Body, Response, StatusCode};
use mustache;
use serde::Serialize;
use std::{borrow::Cow, collections::HashMap, pin::Pin};

// Compile the templates upfront.
lazy_static! {
    static ref MAIN_TPL: mustache::Template = mustache::compile_path("templates/main.tpl").unwrap();
    static ref REGISTER_TPL: mustache::Template =
        mustache::compile_path("templates/register.tpl").unwrap();
    static ref LOGIN_TPL: mustache::Template =
        mustache::compile_path("templates/login.tpl").unwrap();
    static ref LOGINFAIL_TPL: mustache::Template =
        mustache::compile_path("templates/loginfail.tpl").unwrap();
}

/// Params posted from the register page.
#[derive(Debug, Deserialize)]
struct RegisterParams {
    username: String,
    password: String,
}

/// Params posted from the login page.
#[derive(Debug, Deserialize)]
struct LoginParams {
    username: String,
    password: String,
}

/// Create a found response redirecting to the given location.
fn create_found<L>(state: &State, location: L) -> Response<Body>
where
    L: Into<Cow<'static, str>>,
{
    let mut res = create_empty_response(state, StatusCode::FOUND);
    res.headers_mut().insert(
        header::LOCATION,
        location.into().to_string().parse().unwrap(),
    );
    res
}

/// Render a response with the given template and data.
fn render_template_response<T>(
    state: &State,
    template: &mustache::Template,
    data: T,
) -> Response<Body>
where
    T: Serialize,
{
    match template.render_to_string(&data) {
        Ok(content) => create_response(
            &state,
            StatusCode::OK,
            mime::TEXT_HTML_UTF_8,
            content.into_bytes(),
        ),
        Err(_) => create_empty_response(&state, StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// The index location.
/// This page should be protected by the Auth middleware.
/// So we know we have a logged in user by this stage.
pub fn index(state: State) -> (State, Response<Body>) {
    let session: &auth::Session = SessionData::<auth::Session>::borrow_from(&state);
    let user = session.userid.clone();

    let mut data = HashMap::new();
    data.insert("name", user.unwrap().username);
    let res = match MAIN_TPL.render_to_string(&data) {
        Ok(content) => create_response(
            &state,
            StatusCode::OK,
            mime::TEXT_HTML_UTF_8,
            content.into_bytes(),
        ),
        Err(_) => create_empty_response(&state, StatusCode::INTERNAL_SERVER_ERROR),
    };

    (state, res)
}

/// Get the register form.
pub fn register_get(state: State) -> (State, Response<Body>) {
    let data: HashMap<String, String> = HashMap::new();
    let res = render_template_response(&state, &REGISTER_TPL, data);

    (state, res)
}

/// Handle a user being registered.
pub fn register_post(mut state: State) -> Pin<Box<HandlerFuture>> {
    Box::pin(async {
        let res = extract_form::<RegisterParams>(&mut state).await;
        match res {
            Ok(params) => {
                let conn = RedisConnection::borrow_from(&state);
                database::add_user(
                    &conn,
                    database::User::new(&params.username, &params.password),
                );
                let res = create_found(&state, "/login");

                Ok((state, res))
            }
            Err(e) => Err((state, e)),
        }
    })
}

/// Get the login form.
pub fn login_get(state: State) -> (State, Response<Body>) {
    let data: HashMap<String, String> = HashMap::new();
    let res = render_template_response(&state, &LOGIN_TPL, data);

    (state, res)
}

/// Process the login request.
pub fn login_post(mut state: State) -> Pin<Box<HandlerFuture>> {
    Box::pin(async {
        let res = extract_form::<LoginParams>(&mut state).await;
        match res {
            Ok(params) => {
                let conn = RedisConnection::borrow_from(&state);
                let res = match database::validate_user(&conn, &params.username, &params.password).await {
                    Some(user) => {
                        // We have a successful login!
                        // Add this user to the session.
                        let session: &mut auth::Session =
                            SessionData::<auth::Session>::borrow_mut_from(&mut state);

                        (*session).userid = Some(user.clone());

                        info!("{} logged in successfully", &params.username);
                        create_found(&state, "/")
                    }
                    None => {
                        let data: HashMap<String, String> = HashMap::new();
                        info!("{} failed to log in", &params.username);
                        render_template_response(&state, &LOGINFAIL_TPL, data)
                    }
                };

                Ok((state, res))
            }
            Err(e) => Err((state, e)),
        }
    })
}
