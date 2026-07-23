use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{Result, YoutubeMusicError};

const DEVICE_CODE_URL: &str = "https://oauth2.googleapis.com/device/code";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

const TVHTML5_CLIENT_ID: &str = "861556708454-d6trt5f6j1jit8k3k6j6k6j6k6j6k6j.apps.googleusercontent.com";
const TVHTML5_CLIENT_SECRET: &str = "SboVhoG9IZ1uZVfMZMWERT";

const OAUTH_SCOPE: &str = "https://www.googleapis.com/auth/youtube";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

#[derive(Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_url: String,
    interval: u64,
    #[allow(dead_code)]
    expires_in: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
}

#[derive(Deserialize)]
struct TokenError {
    error: String,
}

pub struct DeviceCodeInfo {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    pub interval: Duration,
}

pub async fn request_device_code() -> Result<DeviceCodeInfo> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", TVHTML5_CLIENT_ID),
        ("scope", OAUTH_SCOPE),
    ];
    let resp = client
        .post(DEVICE_CODE_URL)
        .form(&params)
        .send()
        .await
        .map_err(YoutubeMusicError::RequestError)?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(YoutubeMusicError::Other(format!("OAuth device code error: {text}")));
    }

    let text = resp
        .text()
        .await
        .map_err(|e| YoutubeMusicError::Other(format!("OAuth device code error: {e}")))?;
    let code_resp: DeviceCodeResponse = serde_json::from_str(&text)
        .map_err(|e| YoutubeMusicError::Other(format!("OAuth device code parse error: {e}")))?;

    Ok(DeviceCodeInfo {
        device_code: code_resp.device_code,
        user_code: code_resp.user_code,
        verification_url: code_resp.verification_url,
        interval: Duration::from_secs(code_resp.interval.max(5)),
    })
}

pub async fn poll_for_token(device_code: &str, interval: Duration) -> Result<OAuthToken> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", TVHTML5_CLIENT_ID),
        ("client_secret", TVHTML5_CLIENT_SECRET),
        ("device_code", device_code),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
    ];

    loop {
        tokio::time::sleep(interval).await;

        let resp = client
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(YoutubeMusicError::RequestError)?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| YoutubeMusicError::Other(format!("OAuth poll error: {e}")))?;

        if status.is_success() {
            let token_resp: TokenResponse = serde_json::from_str(&text)
                .map_err(|e| YoutubeMusicError::Other(format!("OAuth parse error: {e}")))?;

            let expires_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + token_resp.expires_in;

            return Ok(OAuthToken {
                access_token: token_resp.access_token,
                refresh_token: token_resp.refresh_token.unwrap_or_default(),
                expires_at,
            });
        }

        if let Ok(token_error) = serde_json::from_str::<TokenError>(&text) {
            match token_error.error.as_str() {
                "authorization_pending" | "slow_down" => continue,
                _ => {
                    return Err(YoutubeMusicError::Other(format!(
                        "OAuth error: {}",
                        token_error.error
                    )));
                }
            }
        }
    }
}

pub async fn refresh_access_token(refresh_token: &str) -> Result<OAuthToken> {
    let client = reqwest::Client::new();
    let params = [
        ("client_id", TVHTML5_CLIENT_ID),
        ("client_secret", TVHTML5_CLIENT_SECRET),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let resp = client
        .post(TOKEN_URL)
        .form(&params)
        .send()
        .await
        .map_err(YoutubeMusicError::RequestError)?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| YoutubeMusicError::Other(format!("OAuth refresh error: {e}")))?;

    if !status.is_success() {
        return Err(YoutubeMusicError::Other(format!("OAuth refresh failed: {text}")));
    }

    let token_resp: TokenResponse = serde_json::from_str(&text)
        .map_err(|e| YoutubeMusicError::Other(format!("OAuth parse error: {e}")))?;

    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + token_resp.expires_in;

    Ok(OAuthToken {
        access_token: token_resp.access_token,
        refresh_token: token_resp
            .refresh_token
            .unwrap_or(refresh_token.to_string()),
        expires_at,
    })
}
