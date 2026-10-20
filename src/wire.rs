//! What crosses between a provider and its clients, besides the catalogue.

use serde::{Deserialize, Serialize};

/// The header a request names its lane in; absent, it is interactive.
pub const PRIORITY_HEADER: &str = "x-priority";

/// The header a search names its search box in: unique to whoever types in
/// it, and the same across one burst of typing. A provider that queues
/// searches gives up on one still waiting once a newer one from the same
/// session comes, answering it 409; any other ignores it.
pub const SEARCH_SESSION_HEADER: &str = "x-search-session";

/// Which lane a request waits in when a provider has to ration its
/// requests. Interactive requests, someone at a screen, go ahead of bulk
/// ones, a crawl, an analysis, an index.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    #[default]
    Interactive,
    Bulk,
}

impl Priority {
    pub fn as_str(self) -> &'static str {
        match self {
            Priority::Interactive => "interactive",
            Priority::Bulk => "bulk",
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        match text {
            "interactive" => Ok(Priority::Interactive),
            "bulk" => Ok(Priority::Bulk),
            _ => Err(format!("{text:?} is not a priority: interactive or bulk")),
        }
    }
}

/// How good a stream to ask for. A provider answers with the best it has
/// up to that, which may be less.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    /// MP3, AAC, Opus...: what analysis fetches, and the lightest to play.
    #[default]
    Lossy,
    /// CD quality, 16-bit 44.1 kHz.
    Lossless,
    /// Better than CD.
    Hires,
}

impl Quality {
    pub const ALL: [Quality; 3] = [Quality::Lossy, Quality::Lossless, Quality::Hires];

    pub fn as_str(self) -> &'static str {
        match self {
            Quality::Lossy => "lossy",
            Quality::Lossless => "lossless",
            Quality::Hires => "hires",
        }
    }
}

impl std::str::FromStr for Quality {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        match text {
            "lossy" => Ok(Quality::Lossy),
            "lossless" => Ok(Quality::Lossless),
            "hires" => Ok(Quality::Hires),
            _ => Err(format!(
                "{text:?} is not a quality: lossy, lossless or hires"
            )),
        }
    }
}

/// Up to the asked-for number of a listing's items, and how many there are:
/// more than `items.len()` means the listing was cut short.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: usize,
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            total: 0,
        }
    }
}

/// Where a track's audio is, and what is there, which is not always what
/// was asked for: a track with no lossless master comes lossy, and an
/// account that may not stream a track may get a preview.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileUrl {
    /// Fetched as it is, with no credentials: signed, if it needs to be.
    pub url: String,
    pub quality: Quality,
    pub mime_type: Option<String>,
    /// A preview rather than the track.
    pub sample: bool,
    /// Seconds, of what the URL serves.
    pub duration: Option<f64>,
    /// Kilobits a second, when the file is constant bitrate throughout: then
    /// a byte range is a time range, and an excerpt can be fetched alone.
    #[serde(default)]
    pub constant_bitrate: Option<u32>,
}

/// Which of the catalogue's kinds a listing or an import is of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Track,
    Album,
    Artist,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Track => "track",
            Kind::Album => "album",
            Kind::Artist => "artist",
        }
    }
}

impl std::str::FromStr for Kind {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        match text {
            "track" | "tracks" => Ok(Kind::Track),
            "album" | "albums" => Ok(Kind::Album),
            "artist" | "artists" => Ok(Kind::Artist),
            _ => Err(format!("{text:?} is not a kind: track, album or artist")),
        }
    }
}

/// What a provider is and can do. Search, tracks, albums, artists and
/// stream URLs every provider answers; the rest only where it says so, and
/// a call it cannot answer is a 404.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Short and stable, `qobuz`, `local`: what its references are prefixed
    /// with.
    pub name: String,
    /// For people: "Qobuz", "Music folder".
    pub title: String,
    /// The qualities it can stream in, worst first.
    pub qualities: Vec<Quality>,
    /// `artists/{id}/similar`.
    #[serde(default)]
    pub similar_artists: bool,
    /// `favourites/{kind}`, the account's own likes.
    #[serde(default)]
    pub favourites: bool,
    /// `playlists` and `playlists/{id}/tracks`.
    #[serde(default)]
    pub playlists: bool,
    /// `POST playlists`.
    #[serde(default)]
    pub export: bool,
    /// Discographies tell an artist's own releases from the rest, so new
    /// releases can be followed.
    #[serde(default)]
    pub releases: bool,
    /// `POST import/{kind}`: items in the provider's own shapes, as an
    /// earlier Stelly stored them.
    #[serde(default)]
    pub import: bool,
}

/// Who the provider is signed in as, where it has an account at all.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub subscription: Option<String>,
}

/// The body of `POST /v1/playlists`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPlaylist {
    pub name: String,
    pub track_ids: Vec<i64>,
}

/// What `POST /v1/playlists` answers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Created {
    pub id: i64,
}

/// What a provider answers with when it cannot do what was asked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub error: String,
}

/// The provider said no: 404 for what it does not have, will not stream or
/// cannot do, 400 for a bad request, 401 for a wrong token, 502 when what is
/// behind it failed, 503 when it is not ready yet.
#[derive(Debug, Clone)]
pub struct ProviderError {
    pub status: u16,
    pub message: String,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (provider {})", self.message, self.status)
    }
}

impl std::error::Error for ProviderError {}
