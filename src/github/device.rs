//! Signing in with the browser through GitHub's device flow: the app shows a short code, the user
//! enters it on github.com, and the app polls until GitHub hands over a token.
//! <https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow>

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::agent;
use crate::constants::github::{
    ACCESS_TOKEN_URL, API_TIMEOUT, DEVICE_CODE_URL, DEVICE_SLOW_DOWN, OAUTH_CLIENT_ID, OAUTH_SCOPES,
};
use crate::constants::text;

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    /// What the user types on GitHub, like `WDJB-MJHT`.
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Deserialize)]
struct TokenAnswer {
    access_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
    interval: Option<u64>,
}

fn post_form<T: for<'de> Deserialize<'de>>(url: &str, form: &[(&str, &str)]) -> Result<T, String> {
    let mut response = agent(API_TIMEOUT)
        .post(url)
        .header("Accept", "application/json")
        .send_form(form.iter().copied())
        .map_err(|error| format!("Could not reach GitHub: {error}"))?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("Could not read GitHub's answer: {error}"))?;
    serde_json::from_str(&body).map_err(|_| format!("Unexpected answer from GitHub: {body}"))
}

pub fn request_code() -> Result<DeviceCode, String> {
    post_form(DEVICE_CODE_URL, &[("client_id", OAUTH_CLIENT_ID), ("scope", OAUTH_SCOPES)])
}

/// Polls until the user approves (the token), declines, the code expires, or `cancelled` is set.
pub fn wait_for_token(code: &DeviceCode, cancelled: &Arc<AtomicBool>) -> Result<String, String> {
    let deadline = Instant::now() + Duration::from_secs(code.expires_in);
    let mut interval = Duration::from_secs(code.interval.max(1));
    loop {
        // Short sleeps, so cancelling takes effect quickly.
        let wake = Instant::now() + interval;
        while Instant::now() < wake {
            if cancelled.load(Ordering::Relaxed) {
                return Err(text::SIGN_IN_CANCELLED.to_owned());
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        if Instant::now() >= deadline {
            return Err(text::SIGN_IN_EXPIRED.to_owned());
        }
        let answer: TokenAnswer = post_form(
            ACCESS_TOKEN_URL,
            &[
                ("client_id", OAUTH_CLIENT_ID),
                ("device_code", &code.device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ],
        )?;
        if let Some(token) = answer.access_token {
            return Ok(token);
        }
        match answer.error.as_deref() {
            Some("authorization_pending") => {}
            Some("slow_down") => {
                interval = answer.interval.map_or(interval + DEVICE_SLOW_DOWN, Duration::from_secs);
            }
            Some("expired_token") => return Err(text::SIGN_IN_EXPIRED.to_owned()),
            Some("access_denied") => return Err(text::SIGN_IN_DENIED.to_owned()),
            _ => {
                return Err(answer
                    .error_description
                    .or(answer.error)
                    .unwrap_or_else(|| "GitHub didn't sign you in.".to_owned()));
            }
        }
    }
}
