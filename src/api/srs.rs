#[cfg(feature = "server")]
use crate::server;
use anyhow::Result;
use dioxus::fullstack::Json;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SrsResponse {
    code: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    msg: String,
}

impl SrsResponse {
    fn rejected(message: impl Into<String>) -> Self {
        Self {
            code: 1,
            msg: message.into(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SrsCallback {
    pub action: String,
    pub app: String,
    pub stream: String,
    #[serde(default)]
    pub param: String,
}

#[post("/api/v1/streams", state: server::AppStateEx)]
pub async fn srs_streams(Json(callback): Json<SrsCallback>) -> Result<SrsResponse> {
    tracing::info!(
        action = callback.action,
        app = callback.app,
        stream = callback.stream,
        "received SRS publish callback"
    );
    if callback.action != "on_publish" || callback.app != "live" {
        return Ok(SrsResponse::default());
    }

    let Some(token) = server::security::query_value(&callback.param, "token") else {
        return Ok(SrsResponse::rejected("missing publish token"));
    };
    let Some(room) = state.db.find_room(&callback.stream)? else {
        return Ok(SrsResponse::rejected("unknown stream"));
    };
    if room.publish_token != token {
        return Ok(SrsResponse::rejected("invalid publish token"));
    }
    Ok(SrsResponse::default())
}

#[post("/api/v1/sessions", state: server::AppStateEx)]
pub async fn srs_sessions(Json(callback): Json<SrsCallback>) -> Result<SrsResponse> {
    tracing::info!(
        action = callback.action,
        app = callback.app,
        stream = callback.stream,
        "received SRS start play callback"
    );
    if callback.action != "on_play" || callback.app != "live" {
        return Ok(SrsResponse::default());
    }
    let Some(room) = state.db.find_room(&callback.stream)? else {
        return Ok(SrsResponse::rejected("unknown stream"));
    };
    if let Some(lock_password_hash) = &room.lock_password_hash {
        if !server::security::verify_password(
            server::security::query_value(&callback.param, "password")
                .ok_or(anyhow::anyhow!("room password missing"))?,
            lock_password_hash,
        )? {
            return Ok(SrsResponse::rejected("viewer password required"));
        }
    }
    Ok(SrsResponse::default())
}

#[post("/api/v1/play", state: server::AppStateEx)]
pub async fn srs_play(Json(callback): Json<SrsCallback>) -> Result<SrsResponse> {
    tracing::info!(
        action = callback.action,
        app = callback.app,
        stream = callback.stream,
        "received SRS play callback"
    );
    if !(callback.action == "on_dvr" || callback.action == "on_hls") || callback.app != "live" {
        return Ok(SrsResponse::default());
    }
    let Some(room) = state.db.find_room(&callback.stream)? else {
        return Ok(SrsResponse::rejected("unknown stream"));
    };
    if let Some(lock_password_hash) = &room.lock_password_hash {
        if !server::security::verify_password(
            server::security::query_value(&callback.param, "password")
                .ok_or(anyhow::anyhow!("room password missing"))?,
            lock_password_hash,
        )? {
            return Ok(SrsResponse::rejected("viewer password required"));
        }
    }
    Ok(SrsResponse::default())
}
