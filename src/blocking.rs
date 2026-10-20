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
                for (name, value) in &call.headers {
                    request = request.header(*name, value);
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
                for (name, value) in &call.headers {
                    request = request.header(*name, value);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api;
    use crate::wire::SEARCH_SESSION_HEADER;
    use std::io::{Read, Write};

    /// A call's own headers reach the provider beside the lane.
    #[test]
    fn a_calls_headers_are_sent() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let provider = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buf = [0; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                let n = stream.read(&mut buf).unwrap();
                request.extend_from_slice(&buf[..n]);
            }
            let body = r#"{"tracks":[],"albums":[],"artists":[]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            String::from_utf8(request).unwrap().to_lowercase()
        });
        let client = Client::new(Config { url, token: None });
        client
            .send(api::search("a", 1).header(SEARCH_SESSION_HEADER, "box"))
            .unwrap();
        let request = provider.join().unwrap();
        assert!(request.contains("x-search-session: box\r\n"), "{request}");
        assert!(request.contains("x-priority: interactive\r\n"), "{request}");
    }
}
