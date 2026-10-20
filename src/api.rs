//! Every call a provider answers, named once: a provider routes these
//! paths, and either client sends what these build.

use crate::types::{Album, AlbumTracks, Artist, Playlist, SearchResults, Track};
use crate::wire::{Account, Capabilities, Created, FileUrl, Kind, NewPlaylist, Page, Quality};
use serde_json::Value;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

/// One request to a provider, and what it answers with.
#[derive(Debug, Clone)]
pub struct Call<T> {
    pub method: Method,
    /// From the provider's root, `/v1/...`; ids are percent-encoded.
    pub path: String,
    pub query: Vec<(&'static str, String)>,
    /// Sent as they are, besides the token and the lane.
    pub headers: Vec<(&'static str, String)>,
    pub body: Option<Value>,
    answer: PhantomData<fn() -> T>,
}

impl<T> Call<T> {
    fn get(path: String) -> Self {
        Self {
            method: Method::Get,
            path,
            query: Vec::new(),
            headers: Vec::new(),
            body: None,
            answer: PhantomData,
        }
    }

    fn post(path: String, body: Value) -> Self {
        Self {
            method: Method::Post,
            path,
            query: Vec::new(),
            headers: Vec::new(),
            body: Some(body),
            answer: PhantomData,
        }
    }

    fn query(mut self, key: &'static str, value: impl ToString) -> Self {
        self.query.push((key, value.to_string()));
        self
    }

    /// The same call with a header of the caller's, such as
    /// `wire::SEARCH_SESSION_HEADER`. A provider ignores one it does not
    /// know.
    pub fn header(mut self, name: &'static str, value: impl ToString) -> Self {
        self.headers.push((name, value.to_string()));
        self
    }
}

/// Ids go into paths: anything but letters, digits and `-_.` is encoded, so
/// no id can leave its segment.
fn id(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Whether the provider is up, ready or not.
pub fn health() -> Call<Value> {
    Call::get("/v1/health".into())
}

/// What it is and what it can do.
pub fn capabilities() -> Call<Capabilities> {
    Call::get("/v1/capabilities".into())
}

/// Who it is signed in as.
pub fn account() -> Call<Account> {
    Call::get("/v1/account".into())
}

/// Tracks, albums and artists, up to `limit` of each.
pub fn search(query: &str, limit: usize) -> Call<SearchResults> {
    Call::get("/v1/search".into())
        .query("q", query)
        .query("limit", limit)
}

pub fn track(track_id: i64) -> Call<Track> {
    Call::get(format!("/v1/tracks/{track_id}"))
}

/// Where a track's audio is, in up to `quality`.
pub fn file_url(track_id: i64, quality: Quality) -> Call<FileUrl> {
    Call::get(format!("/v1/tracks/{track_id}/file")).query("quality", quality.as_str())
}

/// An album with its whole tracklist.
pub fn album(album_id: &str) -> Call<AlbumTracks> {
    Call::get(format!("/v1/albums/{}", id(album_id)))
}

/// An artist on their own: name, portrait and biography.
pub fn artist(artist_id: i64) -> Call<Artist> {
    Call::get(format!("/v1/artists/{artist_id}"))
}

/// An artist's discography, newest first, each album with how they are
/// credited on it. `own` keeps only their own albums, EPs, singles and live
/// records, which is what following new releases wants: a much-covered
/// artist's whole listing is mostly other people's.
pub fn artist_releases(artist_id: i64, own: bool, cap: usize) -> Call<Page<Album>> {
    Call::get(format!("/v1/artists/{artist_id}/releases"))
        .query("own", own)
        .query("cap", cap)
}

/// Artists close to this one, by the provider's lights.
pub fn similar_artists(artist_id: i64, limit: usize) -> Call<Vec<Artist>> {
    Call::get(format!("/v1/artists/{artist_id}/similar")).query("limit", limit)
}

pub fn favourite_tracks(cap: usize) -> Call<Page<Track>> {
    Call::get("/v1/favourites/tracks".into()).query("cap", cap)
}

pub fn favourite_albums(cap: usize) -> Call<Page<Album>> {
    Call::get("/v1/favourites/albums".into()).query("cap", cap)
}

pub fn favourite_artists(cap: usize) -> Call<Page<Artist>> {
    Call::get("/v1/favourites/artists".into()).query("cap", cap)
}

/// The account's playlists.
pub fn playlists(cap: usize) -> Call<Page<Playlist>> {
    Call::get("/v1/playlists".into()).query("cap", cap)
}

/// A playlist's tracks.
pub fn playlist_tracks(playlist_id: i64, cap: usize) -> Call<Page<Track>> {
    Call::get(format!("/v1/playlists/{playlist_id}/tracks")).query("cap", cap)
}

/// A new private playlist of `track_ids`, in order.
pub fn create_playlist(name: &str, track_ids: &[i64]) -> Call<Created> {
    let body = NewPlaylist {
        name: name.to_string(),
        track_ids: track_ids.to_vec(),
    };
    Call::post(
        "/v1/playlists".into(),
        serde_json::to_value(body).expect("a playlist serialises"),
    )
}

/// Items in the provider's own shapes, as it once handed them out, in this
/// protocol's: one answer for each, `None` where it makes nothing of one.
/// `T` is `Track`, `Album` or `Artist`, as `kind` says.
pub fn import<T>(kind: Kind, items: &[Value]) -> Call<Vec<Option<T>>> {
    Call::post(
        format!("/v1/import/{}", kind.as_str()),
        Value::Array(items.to_vec()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_cannot_leave_their_path_segment() {
        assert_eq!(track(123).path, "/v1/tracks/123");
        assert_eq!(album("0060254728690").path, "/v1/albums/0060254728690");
        assert_eq!(album("../user").path, "/v1/albums/..%2Fuser");
        assert_eq!(album("a b").path, "/v1/albums/a%20b");
        assert_eq!(
            search("a b", 5).query,
            [("q", "a b".to_string()), ("limit", "5".to_string())]
        );
        assert_eq!(
            file_url(1, Quality::Lossless).query,
            [("quality", "lossless".to_string())]
        );
    }
}
