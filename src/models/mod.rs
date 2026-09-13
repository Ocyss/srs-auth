use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use native_db::{native_db, ToKey};
#[cfg(feature = "server")]
use native_model::{native_model, Model};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(feature = "server", native_model(id = 1, version = 1), native_db)]
pub struct Account {
    #[cfg_attr(feature = "server", primary_key)]
    pub username: String,
    pub password_hash: String,
    pub is_administrator: bool,
    pub disabled_at: Option<DateTime<Utc>>,
    pub disabled_message: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Account {
    pub fn is_disabled_at(&self, now: DateTime<Utc>) -> bool {
        self.disabled_at
            .is_some_and(|disabled_at| disabled_at > now)
    }
    pub fn room_auth(&self, room: &LiveRoom) -> anyhow::Result<()> {
        if self.is_administrator || self.username == room.owner_username {
            return Ok(());
        }
        Err(anyhow::anyhow!("room owner permission required"))
    }
    #[cfg(feature = "server")]
    pub fn with_new_password(&mut self, new_password: &str) -> anyhow::Result<()> {
        if new_password.len() < 10 {
            return Err(anyhow::anyhow!(
                "new password must be at least 10 characters"
            ));
        }
        self.password_hash = crate::server::security::hash_password(new_password)?;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Default)]
#[cfg_attr(feature = "server", native_model(id = 2, version = 1), native_db)]
pub struct LiveRoom {
    #[cfg_attr(feature = "server", primary_key)]
    pub stream_key: String,
    #[cfg_attr(feature = "server", secondary_key)]
    pub owner_username: String,
    pub title: String,
    pub publish_token: String,

    pub lock_password_hash: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
