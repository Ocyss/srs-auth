use std::env;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Clone)]
pub struct SrsApiClient {
    _client: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct StreamsResponse {
    code: i32,
    streams: Vec<SrsStream>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SrsStream {
    pub app: String,
    pub name: String,
    pub clients: i64,
    pub live_ms: i64,
    pub send_bytes: i64,
    pub recv_bytes: i64,
    pub publish: SrsPublish,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SrsPublish {
    pub active: bool,
}

impl SrsApiClient {
    pub fn new() -> Self {
        Self {
            _client: reqwest::Client::new(),
        }
    }

    fn request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        let mut req = self
            ._client
            .request(method, format!("{}{}", *super::SRS_API_URL, url));
        if let Ok(username) = env::var("SRS_API_AUTH_USERNAME") {
            req = req.basic_auth(username, env::var("SRS_API_AUTH_PASSWORD").ok());
        }
        req
    }

    pub async fn streams(&self) -> Result<Vec<SrsStream>> {
        let response = self
            .request(reqwest::Method::GET, "/api/v1/streams/")
            .send()
            .await
            .context("failed to request SRS streams API")?
            .error_for_status()
            .context("SRS streams API returned an error response")?
            .text()
            .await
            .context("failed to read SRS streams API response text")?;

        let response = serde_json::from_str::<StreamsResponse>(&response)
            .context("failed to parse SRS streams API response")?;

        if response.code != 0 {
            anyhow::bail!("SRS streams API returned code {}", response.code);
        }
        Ok(response.streams)
    }
}
