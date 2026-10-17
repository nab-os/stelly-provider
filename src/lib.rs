//! What Stelly asks a music provider for, and how.
//!
//! A provider is a service in front of a music catalogue: a streaming
//! account, a folder of files. It answers the same few questions whatever is
//! behind it, search, a track, an album, an artist, where a track's audio
//! is, in `types`' shapes, over HTTP; Stelly asks them, and never learns what
//! the provider is.
//!
//! ```ignore
//! let provider = stelly_provider::Client::new(Config::from_env(None));
//! let album = provider.send(api::album("0060254728690")).await?;
//! let file = provider.at(Priority::Bulk).send(api::file_url(12345, Quality::Lossy)).await?;
//! ```
//!
//! `api` names every call once, and both clients send any of them; the
//! `async` and `blocking` features add them. A provider implements the
//! routes `api` builds, answering `wire::Problem` when it cannot.

pub mod api;
pub mod config;
pub mod types;
pub mod wire;

#[cfg(feature = "async")]
mod async_client;
#[cfg(feature = "blocking")]
pub mod blocking;

#[cfg(feature = "async")]
pub use async_client::Client;
pub use config::Config;
pub use types::{
    recent_releases, Album, AlbumTracks, Artist, Credit, Playlist, ReleaseKind, SearchResults,
    Track, TrackIdentity,
};
pub use wire::{Account, Capabilities, FileUrl, Kind, Page, Priority, ProviderError, Quality};
