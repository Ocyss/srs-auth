use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use std::sync::LazyLock;
pub(crate) mod db;
pub(crate) mod security;
pub(crate) mod srs_api;

pub static SRS_API_URL: LazyLock<String> =
    LazyLock::new(|| environment_url("SRS_API_URL", "http://127.0.0.1:1985"));

pub static RTMP_BASE_URL: LazyLock<String> =
    LazyLock::new(|| environment_url("RTMP_BASE_URL", "rtmp://127.0.0.1:1935"));

pub static SRT_BASE_URL: LazyLock<String> =
    LazyLock::new(|| environment_url("SRT_BASE_URL", "srt://127.0.0.1:10080"));

pub static HTTP_FLV_BASE_URL: LazyLock<String> =
    LazyLock::new(|| environment_url("HTTP_FLV_BASE_URL", "http://127.0.0.1:8080"));

pub static WEB_RTC_BASE_URL: LazyLock<String> =
    LazyLock::new(|| environment_url("WEB_RTC_BASE_URL", "http://127.0.0.1:1985/rtc/v1"));

pub struct AppState {
    pub db: db::Db<'static>,
    pub srs_api: srs_api::SrsApiClient,
}

impl AppState {
    pub async fn new() -> anyhow::Result<Self> {
        let app_state = Self {
            db: db::init_db().await?,
            srs_api: srs_api::SrsApiClient::new(),
        };
        bootstrap_administrator(&app_state)?;
        Ok(app_state)
    }
}

pub type AppStateEx = dioxus::server::axum::Extension<Arc<crate::server::AppState>>;

#[derive(Clone)]
pub struct AuthEx {
    pub account: crate::models::Account,
    pub session_claims: security::SessionClaims,
}

impl<S> axum::extract::FromRequestParts<S> for AuthEx
where
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request_parts(
        req: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        req.extensions.get::<AuthEx>().cloned().ok_or_else(|| {
            axum::response::Response::builder()
                .status(axum::http::StatusCode::UNAUTHORIZED)
                .body("Not authorized".into())
                .unwrap()
        })
    }
}

impl<S> axum::extract::OptionalFromRequestParts<S> for AuthEx
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        req: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(req.extensions.get::<AuthEx>().cloned())
    }
}

fn environment_url(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_owned())
        .trim_end_matches('/')
        .to_owned()
}

fn bootstrap_administrator(app_state: &AppState) -> Result<()> {
    if app_state
        .db
        .list_accounts()?
        .iter()
        .any(|account| account.is_administrator)
    {
        return Ok(());
    }

    let (username, password) = administrator_credentials()?;
    let _ = create_account_and_room(
        &app_state.db,
        &username,
        &password,
        &format!("{}的直播间", username),
        true,
    )?;

    tracing::info!(username, "created initial administrator");
    Ok(())
}

pub fn create_account_and_room(
    db: &crate::server::db::Db,
    username: &str,
    password: &str,
    title: &str,
    is_administrator: bool,
) -> Result<(crate::models::Account, crate::models::LiveRoom)> {
    let now = Utc::now();
    let account = crate::models::Account {
        username: username.to_owned(),
        password_hash: security::hash_password(password)?,
        is_administrator,
        disabled_at: None,
        disabled_message: None,
        created_at: now,
        updated_at: now,
    };
    let room = crate::models::LiveRoom {
        stream_key: security::generate_identifier(),
        owner_username: username.to_owned(),
        title: title.to_owned(),
        publish_token: security::generate_token(),
        lock_password_hash: None,
        created_at: now,
        updated_at: now,
    };
    let transaction = db.rw_transaction()?;
    transaction.insert(account.clone())?;
    transaction.insert(room.clone())?;
    transaction.commit()?;
    Ok((account, room))
}

fn administrator_credentials() -> Result<(String, String)> {
    match (
        std::env::var("ADMIN_USERNAME"),
        std::env::var("ADMIN_PASSWORD"),
    ) {
        (Ok(username), Ok(password)) => {
            let username = username.trim().to_owned();
            let password = password.trim().to_owned();
            if username.is_empty() || password.is_empty() {
                return Err(anyhow!(
                    "ADMIN_USERNAME and ADMIN_PASSWORD must not be empty"
                ));
            }
            Ok((username, password))
        }
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => Err(anyhow!(
            "ADMIN_USERNAME and ADMIN_PASSWORD must be configured together"
        )),
        _ => {
            let password = security::generate_initial_password();
            tracing::warn!(password, "not set, using default admin credentials");
            Ok(("admin".to_owned(), password))
        }
    }
}
