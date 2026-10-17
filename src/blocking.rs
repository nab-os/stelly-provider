//! A provider's client for blocking callers, on ureq.

use crate::api::{Call, Method};
use crate::wire::{Problem, ProviderError, PRIORITY_HEADER};
use crate::{Config, Priority};
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::time::Duration;

#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
    config: Config,
    priority: Priority,
}

impl Client {
    pub fn new(config: Config) -> Self {
        let agent = ureq::Agent::config_builder()
            // No overall timeout: a bulk request can wait its turn behind a
            // queue of others, and the provider gives up on its own.
            .timeout_connect(Some(Duration::from_secs(5)))
            .http_status_as_error(false)
            .build()
            .new_agent();
        Self {
            agent,
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

    pub fn send<T: DeserializeOwned>(&self, call: Call<T>) -> Result<T> {
        let url = format!("{}{}", self.config.url, call.path);
        let auth = self.config.token.as_ref().map(|t| format!("Bearer {t}"));
        let unreachable = || {
            format!(
                "reaching the music provider at {}; is it running?",
                self.config.url
            )
        };

        let mut response = match call.method {
            Method::Get => {
                let mut request = self
                    .agent
                    .get(&url)
                    .header(PRIORITY_HEADER, self.priority.as_str());
                for (key, value) in &call.query {
                    request = request.query(*key, value);
                }
                if let Some(auth) = &auth {
                    request = request.header("Authorization", auth);
                }
                request.call().with_context(unreachable)?
            }
            Method::Post => {
                let mut request = self
                    .agent
                    .post(&url)
                    .header(PRIORITY_HEADER, self.priority.as_str());
                for (key, value) in &call.query {
                    request = request.query(*key, value);
                }
                if let Some(auth) = &auth {
                    request = request.header("Authorization", auth);
                }
                let body = call.body.clone().unwrap_or(serde_json::Value::Null);
                request.send_json(&body).with_context(unreachable)?
            }
        };

        let status = response.status();
        let body = response
            .body_mut()
            .read_to_string()
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
