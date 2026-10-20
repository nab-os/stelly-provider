//! A provider's client for async callers, on reqwest.

use crate::api::{Call, Method};
use crate::wire::{Problem, ProviderError, PRIORITY_HEADER};
use crate::{Config, Priority};
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::time::Duration;

/// Cheap to clone; clones share connections.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    config: Config,
    priority: Priority,
}

impl Client {
    pub fn new(config: Config) -> Self {
        Self {
            // No overall timeout: a bulk request can wait its turn behind a
            // queue of others, and the provider gives up on its own.
            http: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .build()
                .expect("a client with a connect timeout builds"),
            config,
            priority: Priority::Interactive,
        }
    }

    /// The same provider, asked to queue in `priority`'s lane.
    pub fn at(&self, priority: Priority) -> Self {
        Self {
            priority,
            ..self.clone()
        }
    }

    pub fn url(&self) -> &str {
        &self.config.url
    }

    pub async fn send<T: DeserializeOwned>(&self, call: Call<T>) -> Result<T> {
        let url = format!("{}{}", self.config.url, call.path);
        let mut request = match call.method {
            Method::Get => self.http.get(&url),
            Method::Post => self.http.post(&url),
        }
        .query(&call.query)
        .header(PRIORITY_HEADER, self.priority.as_str());
        if let Some(token) = &self.config.token {
            request = request.bearer_auth(token);
        }
        for (name, value) in &call.headers {
            request = request.header(*name, value);
        }
        if let Some(body) = &call.body {
            request = request.json(body);
        }

        let response = request.send().await.with_context(|| {
            format!(
                "reaching the music provider at {}; is it running?",
                self.config.url
            )
        })?;
        let status = response.status();
        let body = response
            .text()
            .await
            .with_context(|| format!("reading {}", call.path))?;
        if !status.is_success() {
            let message = serde_json::from_str::<Problem>(&body).map_or(body, |p| p.error);
            return Err(ProviderError {
                status: status.as_u16(),
                message,
            }
            .into());
        }
        serde_json::from_str(&body)
            .with_context(|| format!("parsing the provider's answer to {}", call.path))
    }
}
