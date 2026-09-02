//! The Roblox experience-questionnaire endpoints.
//!
//! None of these appear in Roblox's published Open Cloud specification. They
//! are the endpoints the Creator Hub itself calls, authenticated with a
//! `.ROBLOSECURITY` cookie, and they can change without notice. That is the
//! reason this is its own tool rather than a subcommand of an Open Cloud CLI:
//! a spec-drift check cannot warn about an endpoint that is not in the spec, so
//! breakage here should cost one small release and nothing else.

use anyhow::{bail, Context, Result};
use reqwest::{Client, Method, Response, StatusCode};
use std::time::Duration;

use crate::model::{
    Answer, Latest, PreviewResponse, Questionnaire, SubmitResponse, ValidateResponse, Wrapper,
};

const BASE: &str = "https://apis.roblox.com/experience-questionnaire/v1";
const LOCALE: &str = "en_us";

#[derive(Debug)]
pub struct Api {
    http: Client,
    cookie: String,
    /// Roblox hands this out on the first rejected write and expects it back on
    /// every subsequent one.
    csrf: std::sync::Mutex<Option<String>>,
}

impl Api {
    pub fn new(cookie: String) -> Result<Self> {
        let http = Client::builder()
            .gzip(true)
            .timeout(Duration::from_secs(60))
            .build()
            .context("building the HTTP client")?;

        Ok(Self {
            http,
            cookie,
            csrf: std::sync::Mutex::new(None),
        })
    }

    /// The id of the questionnaire this universe is currently being asked.
    pub async fn latest_id(&self, universe_id: u64) -> Result<String> {
        let url = format!("{BASE}/questionnaires/{universe_id}/latest");
        let latest: Latest = self.request_json(Method::GET, &url, None).await?;
        Ok(latest.questionnaire_id)
    }

    /// The questions themselves.
    pub async fn questionnaire(&self, id: &str) -> Result<Questionnaire> {
        let url = format!("{BASE}/questionnaires/{id}?localeCode={LOCALE}");
        let wrapper: Wrapper = self.request_json(Method::GET, &url, None).await?;
        Ok(wrapper.questionnaire)
    }

    /// Roblox's own opinion on whether these answers are complete.
    pub async fn validate(&self, id: &str, answers: &[Answer]) -> Result<ValidateResponse> {
        let url = format!("{BASE}/responses/validate");
        let body = serde_json::json!({
            "questionnaireId": id,
            "response": { "answers": answers },
        });
        self.request_json(Method::POST, &url, Some(body)).await
    }

    /// What rating these answers would produce, without filing them.
    pub async fn preview(
        &self,
        universe_id: u64,
        id: &str,
        answers: &[Answer],
    ) -> Result<PreviewResponse> {
        let url = format!("{BASE}/responses/preview?localeCode={LOCALE}");
        let body = serde_json::json!({
            "universeId": universe_id,
            "questionnaireId": id,
            "response": { "answers": answers },
        });
        self.request_json(Method::POST, &url, Some(body)).await
    }

    /// File the declaration.
    ///
    /// The id is re-read first. A questionnaire that changed between planning
    /// and submitting would mean filing answers to questions that are no longer
    /// the ones being asked, and Roblox would take them.
    pub async fn submit(
        &self,
        universe_id: u64,
        id: &str,
        answers: &[Answer],
    ) -> Result<SubmitResponse> {
        let current = self.latest_id(universe_id).await?;
        if current != id {
            bail!(
                "the questionnaire changed while this ran (planned against {id}, \
                 Roblox now asks {current}); run `pull` and review before submitting"
            );
        }

        let url = format!("{BASE}/responses/{universe_id}/submissions");
        let body = serde_json::json!({
            "questionnaireId": id,
            "response": { "answers": answers },
        });
        self.request_json(Method::POST, &url, Some(body)).await
    }

    async fn request_json<T: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        url: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        let response = self.send(method, url, body).await?;
        let status = response.status();
        let text = response.text().await.context("reading the response body")?;

        serde_json::from_str(&text).with_context(|| {
            format!(
                "the response to {url} was not the JSON this tool expects \
                 (HTTP {status}). These endpoints are undocumented and Roblox \
                 can change them without notice. Body: {}",
                truncate(&text)
            )
        })
    }

    async fn send(
        &self,
        method: Method,
        url: &str,
        body: Option<serde_json::Value>,
    ) -> Result<Response> {
        // Two attempts, and only two: the first can be rejected for a stale or
        // absent CSRF token, which the rejection itself hands back. A second
        // rejection is a real one.
        for attempt in 0..2 {
            let mut request = self
                .http
                .request(method.clone(), url)
                .header("Cookie", format!(".ROBLOSECURITY={}", self.cookie));

            if let Some(token) = self.csrf.lock().expect("csrf mutex").as_ref() {
                request = request.header("x-csrf-token", token);
            }
            if let Some(json) = &body {
                request = request.json(json);
            }

            let response = request
                .send()
                .await
                .with_context(|| format!("calling {url}"))?;

            if response.status() == StatusCode::FORBIDDEN {
                if let Some(token) = response.headers().get("x-csrf-token") {
                    let token = token.to_str().unwrap_or_default().to_string();
                    *self.csrf.lock().expect("csrf mutex") = Some(token);
                    if attempt == 0 {
                        continue;
                    }
                }
                let text = response.text().await.unwrap_or_default();
                bail!(
                    "Roblox refused this request (403). The cookie is usually the \
                     reason: expired, or belonging to an account without edit \
                     access to this universe. Body: {}",
                    truncate(&text)
                );
            }

            if !response.status().is_success() {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                bail!(
                    "Roblox returned HTTP {status} for {url}: {}",
                    truncate(&text)
                );
            }

            return Ok(response);
        }

        bail!("gave up after two CSRF attempts on {url}")
    }
}

fn truncate(text: &str) -> String {
    const LIMIT: usize = 400;
    if text.len() <= LIMIT {
        return text.to_string();
    }
    // On a char boundary, so a multi-byte body does not panic the error path.
    let end = text
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|i| *i <= LIMIT)
        .last()
        .unwrap_or(0);
    format!("{}...", &text[..end])
}

/// Where the cookie comes from, in the order a person expects.
///
/// The flag beats the environment beats the local Studio install, so that a
/// deliberate choice always wins over an ambient one. The Studio fallback is
/// what makes the tool usable without ever putting a cookie in a shell history.
pub fn resolve_cookie(flag: Option<String>) -> Result<String> {
    if let Some(cookie) = flag.filter(|c| !c.is_empty()) {
        return Ok(cookie);
    }

    if let Ok(cookie) = std::env::var("RBX_COOKIE") {
        if !cookie.is_empty() {
            return Ok(cookie);
        }
    }

    rbx_cookie::get_value().context(
        "no cookie: pass --cookie, set RBX_COOKIE, or sign in to Roblox Studio \
         on this machine so it can be read from there",
    )
}
