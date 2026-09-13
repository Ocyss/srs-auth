use std::ops::Deref;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::server;

use crate::models;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Default)]
pub struct RoomSummary {
    pub room: models::LiveRoom,
    pub is_live: bool,
    pub is_locked: bool,
    pub viewer_count: i64,
    pub play_url: Option<(String, String, String, String)>,
}

impl Deref for RoomSummary {
    type Target = models::LiveRoom;

    fn deref(&self) -> &Self::Target {
        &self.room
    }
}

impl RoomSummary {
    #[cfg(feature = "server")]
    fn from_room(room: models::LiveRoom, stream: Option<&server::srs_api::SrsStream>) -> Self {
        Self {
            room: models::LiveRoom {
                publish_token: "".to_string(),
                lock_password_hash: None,
                ..room.clone()
            },
            is_live: stream.is_some_and(|stream| stream.publish.active),
            is_locked: room.lock_password_hash.is_some(),
            viewer_count: stream.map_or(0, |stream| stream.clients),
            play_url: Some((
                format!(
                    "{}?streamid=#!::r=live/{},m=request",
                    server::SRT_BASE_URL.as_str(),
                    room.stream_key
                ),
                format!(
                    "{}/live/{}",
                    server::RTMP_BASE_URL.as_str(),
                    room.stream_key
                ),
                format!(
                    "{}/live/{}.flv",
                    server::HTTP_FLV_BASE_URL.as_str(),
                    room.stream_key
                ),
                format!(
                    "{}/whep/?app=live&stream={}",
                    server::WEB_RTC_BASE_URL.as_str(),
                    room.stream_key
                ),
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateRoomRequest {
    pub title: Option<String>,
    pub lock_password: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ViewerAccessRequest {
    pub stream_key: String,
    pub password: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PublishInfoResponse {
    pub rtmp_url: String,
    pub srt_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateRoomRequest {
    pub owner_username: String,
    pub title: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CreateRoomResponse {
    pub owner_username: String,
    pub initial_password: String,
    pub stream_key: String,
    pub publish_info: PublishInfoResponse,
}

#[cfg(feature = "server")]
impl From<&models::LiveRoom> for PublishInfoResponse {
    fn from(room: &models::LiveRoom) -> Self {
        Self {
            rtmp_url: format!(
                "{}/live/{}?token={}",
                server::RTMP_BASE_URL.as_str(),
                room.stream_key,
                room.publish_token
            ),
            srt_url: format!(
                "{}?streamid=#!::r=live/{},m=publish,token={}",
                server::SRT_BASE_URL.as_str(),
                room.stream_key,
                room.publish_token
            ),
        }
    }
}

fn validate_create_room_request(request: &CreateRoomRequest) -> Result<(String, String)> {
    let owner_username = request.owner_username.trim();
    let title = request.title.trim();
    if owner_username.len() < 2 || owner_username.len() > 64 || title.is_empty() {
        return Err(anyhow!(
            "username must be 2-64 characters and room title is required"
        ));
    }
    Ok((owner_username.to_owned(), title.to_owned()))
}

#[get("/api/rooms", state: server::AppStateEx)]
pub async fn list_rooms() -> Result<Vec<RoomSummary>> {
    let rooms = state.db.list_rooms()?;
    let streams = match state.srs_api.streams().await {
        Ok(streams) => streams,
        Err(error) => {
            tracing::warn!(?error, "failed to load SRS stream status");
            Vec::new()
        }
    };

    let mut summaries = rooms
        .into_iter()
        .map(|room| {
            let stream = streams
                .iter()
                .find(|stream| stream.app == "live" && stream.name == room.stream_key);
            RoomSummary::from_room(room, stream)
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        right
            .is_live
            .cmp(&left.is_live)
            .then_with(|| left.room.title.cmp(&right.room.title))
    });
    Ok(summaries)
}

#[post("/api/rooms", state: server::AppStateEx, auth: server::AuthEx)]
pub async fn create_room(request: CreateRoomRequest) -> Result<CreateRoomResponse> {
    if !auth.account.is_administrator {
        return Err(anyhow!("administrator permission required"));
    }
    let (owner_username, title) = validate_create_room_request(&request)?;
    if state.db.find_account(&owner_username)?.is_some() {
        return Err(anyhow!("username already exists"));
    }

    let initial_password = server::security::generate_initial_password();
    let (_account, room) = server::create_account_and_room(
        &state.db,
        &owner_username,
        &initial_password,
        &title,
        false,
    )?;

    Ok(CreateRoomResponse {
        owner_username,
        initial_password,
        stream_key: room.stream_key.clone(),
        publish_info: (&room).into(),
    })
}

#[cfg(test)]
mod tests {
    use super::{validate_create_room_request, CreateRoomRequest};

    #[test]
    fn create_room_request_trims_valid_fields() {
        let request = CreateRoomRequest {
            owner_username: "  alice  ".to_owned(),
            title: "  Alice's room  ".to_owned(),
        };

        assert_eq!(
            validate_create_room_request(&request).unwrap(),
            ("alice".to_owned(), "Alice's room".to_owned())
        );
    }

    #[test]
    fn create_room_request_rejects_invalid_owner_or_title() {
        for request in [
            CreateRoomRequest {
                owner_username: "ab".to_owned(),
                title: "A room".to_owned(),
            },
            CreateRoomRequest {
                owner_username: "alice".to_owned(),
                title: " ".to_owned(),
            },
        ] {
            assert!(validate_create_room_request(&request).is_err());
        }
    }
}

#[patch("/api/rooms/{stream_key}/update", state: server::AppStateEx, auth: server::AuthEx)]
pub async fn update_room(stream_key: String, request: UpdateRoomRequest) -> Result<()> {
    let Some(room) = state.db.find_room(&stream_key)? else {
        return Err(anyhow!("room not found"));
    };
    auth.account.room_auth(&room)?;

    let updated_room = crate::models::LiveRoom {
        title: request.title.unwrap_or(room.title).trim().to_owned(),
        lock_password_hash: request
            .lock_password
            .map_or(Ok(room.lock_password_hash), |p| {
                server::security::hash_password(&p).map(Some)
            })?,
        updated_at: Utc::now(),
        ..room
    };
    state.db.upsert_room(updated_room.clone())?;
    Ok(())
}

#[put("/api/rooms/{stream_key}/rotate-token", state: server::AppStateEx, auth: server::AuthEx)]
pub async fn rotate_publish_token(stream_key: String) -> Result<String> {
    let Some(room) = state.db.find_room(&stream_key)? else {
        return Err(anyhow!("room not found"));
    };
    auth.account.room_auth(&room)?;
    let publish_token = server::security::generate_token();
    let updated_room = crate::models::LiveRoom {
        publish_token: publish_token.clone(),
        updated_at: Utc::now(),
        ..room
    };
    state.db.upsert_room(updated_room)?;
    Ok(publish_token)
}

#[get("/api/rooms/{stream_key}/publish-info", state: server::AppStateEx, auth: server::AuthEx)]
pub async fn publish_info(stream_key: String) -> Result<PublishInfoResponse> {
    let Some(room) = state.db.find_room(&stream_key)? else {
        return Err(anyhow!("room not found"));
    };
    auth.account.room_auth(&room)?;

    Ok((&room).into())
}
