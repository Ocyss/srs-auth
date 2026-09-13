use anyhow::Result;
use dioxus::fullstack::{Form, SetCookie, SetHeader};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::Account;
#[cfg(feature = "server")]
use crate::server;

const SESSION_DURATION_DAYS: i64 = 30;
const SESSION_MAX_AGE_SECONDS: i64 = SESSION_DURATION_DAYS * 24 * 60 * 60;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AccountSummary {
    pub username: String,
    pub is_administrator: bool,
    pub disabled_until_unix: Option<i64>,
    pub disabled_message: Option<String>,
}

impl From<Account> for AccountSummary {
    fn from(account: Account) -> Self {
        AccountSummary {
            username: account.username,
            is_administrator: account.is_administrator,
            disabled_until_unix: account.disabled_at.map(|dt| dt.timestamp()),
            disabled_message: account.disabled_message,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateAccountRequest {
    pub session_token: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DisableAccountRequest {
    pub session_token: String,
    pub username: String,
    pub disabled_until_unix: Option<i64>,
    pub disabled_message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[post("/api/auth/login", state: server::AppStateEx)]
pub async fn login(form: Form<LoginForm>) -> Result<SetHeader<SetCookie>> {
    let username = form.username.trim();
    let Some(account) = state.db.find_account(username)? else {
        return Err(anyhow::anyhow!("invalid username or password"));
    };
    if account.is_disabled_at(chrono::Utc::now()) {
        return Err(anyhow::anyhow!("account is temporarily disabled"));
    }
    if !server::security::verify_password(&form.password, &account.password_hash)? {
        return Err(anyhow::anyhow!("invalid username or password"));
    }

    let now = chrono::Utc::now();
    let expires_at = now + chrono::Duration::days(SESSION_DURATION_DAYS);
    let session_token = server::security::create_session_token(&server::security::SessionClaims {
        sub: account.username.clone(),
        exp: expires_at.timestamp() as usize,
    })?;
    Ok(SetHeader::new(format!(
        "token={session_token}; Path=/; Max-Age={SESSION_MAX_AGE_SECONDS}; HttpOnly; SameSite=Lax; Secure"
    ))?)
}

#[post("/api/auth/logout", _auth: server::AuthEx)]
pub async fn logout() -> Result<SetHeader<SetCookie>> {
    Ok(SetHeader::new(
        "token=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax; Secure",
    )?)
}

#[post("/api/auth/me", auth: server::AuthEx)]
pub async fn current_account() -> Result<AccountSummary> {
    Ok(auth.account.into())
}

#[put("/api/auth/password", state: server::AppStateEx, mut auth: server::AuthEx)]
pub async fn change_password(request: ChangePasswordRequest) -> Result<()> {
    if !server::security::verify_password(&request.current_password, &auth.account.password_hash)? {
        return Err(anyhow::anyhow!("current password is incorrect"));
    }

    auth.account.with_new_password(&request.new_password)?;
    state.db.upsert_account(auth.account)?;
    Ok(())
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use chrono::Utc;

    use crate::{models::Account, server::security};

    fn account() -> Account {
        Account {
            username: "alice".to_owned(),
            password_hash: security::hash_password("correct horse battery staple").unwrap(),
            is_administrator: false,
            disabled_at: None,
            disabled_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn password_update_replaces_the_previous_password() {
        let mut updated = account();
        updated
            .with_new_password("new correct battery staple")
            .unwrap();

        assert!(
            security::verify_password("new correct battery staple", &updated.password_hash)
                .unwrap()
        );
        assert!(
            !security::verify_password("correct horse battery staple", &updated.password_hash)
                .unwrap()
        );
    }

    #[test]
    fn password_update_rejects_short_passwords() {
        let mut acc = account();
        assert!(acc.with_new_password("short").is_err());
    }
}

#[post("/api/auth/accounts/disable", state: server::AppStateEx, auth: server::AuthEx)]
pub async fn disable_account(request: DisableAccountRequest) -> Result<AccountSummary> {
    if !auth.account.is_administrator {
        return Err(anyhow::anyhow!("administrator permission required"));
    }
    if auth.account.username == request.username && request.disabled_until_unix.is_some() {
        return Err(anyhow::anyhow!(
            "an administrator cannot disable their own account"
        ));
    }
    let Some(account) = state.db.find_account(&request.username)? else {
        return Err(anyhow::anyhow!("account not found"));
    };
    let disabled_at = request
        .disabled_until_unix
        .map(|timestamp| {
            chrono::DateTime::from_timestamp(timestamp, 0)
                .ok_or_else(|| anyhow::anyhow!("invalid disable expiry"))
        })
        .transpose()?;
    let updated_account = crate::models::Account {
        disabled_at,
        disabled_message: request
            .disabled_message
            .filter(|message| !message.trim().is_empty()),
        ..account
    };
    state.db.upsert_account(updated_account.clone())?;
    Ok(updated_account.into())
}
