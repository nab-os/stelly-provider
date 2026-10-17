//! The catalogue as Stelly sees it, whichever provider it comes from:
//! tracks, albums, artists and playlists, flattened to what the app shows.
//!
//! A provider turns its own shapes into these; nothing past it ever sees
//! them. Ids are the provider's own: a track's and an artist's are numbers,
//! an album's is text.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub artist_id: Option<i64>,
    pub album: String,
    pub album_id: Option<String>,
    /// Seconds.
    pub duration: Option<i64>,
    /// Whether the provider will stream it; greying the ones it will not out
    /// saves a round trip that would only fail.
    pub streamable: bool,
    /// Whether it comes in better than CD quality.
    pub hires: bool,
    /// The album's cover; a track has no art of its own.
    #[serde(default)]
    pub image: Option<String>,
    /// The recording, as opposed to this particular catalogue entry. One
    /// song reached through a single, an album and a deluxe reissue is three
    /// track ids but one ISRC, which is what `identity` dedupes on.
    #[serde(default)]
    pub isrc: Option<String>,
    /// The release this entry belongs to: a full date where the provider has
    /// one, the same shape as `Album::released`.
    #[serde(default)]
    pub released: Option<String>,
    /// The credits, composer, lyricist, producer and so on, each tagged with
    /// their role, semicolon-separated, as the provider reports them.
    #[serde(default)]
    pub performers: Option<String>,
    /// When it was liked, in Unix seconds. Only a likes listing has one.
    #[serde(default)]
    pub liked_at: Option<i64>,
}

impl Track {
    /// What makes two queue entries "the same music".
    ///
    /// The ISRC when there is one, since that names the recording rather
    /// than the release. Falling back to the track id means an untagged track
    /// is only ever a duplicate of itself, which is the safe direction: a
    /// missed duplicate is a nuisance, a wrongly dropped track is a bug.
    pub fn identity(&self) -> TrackIdentity {
        match self
            .isrc
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(isrc) => TrackIdentity::Recording(isrc.to_ascii_uppercase()),
            None => TrackIdentity::Entry(self.id),
        }
    }

    /// `m:ss`, blank when there is no duration.
    pub fn duration_label(&self) -> String {
        match self.duration {
            Some(seconds) if seconds > 0 => format!("{}:{:02}", seconds / 60, seconds % 60),
            _ => String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TrackIdentity {
    /// An ISRC, upper-cased: catalogues are not consistent about the case.
    Recording(String),
    /// A track id, for the tracks that carry no ISRC.
    Entry(i64),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: Option<i64>,
    pub released: Option<String>,
    pub genre: Option<String>,
    pub tracks_count: Option<i64>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    /// When it was liked, in Unix seconds. Only a likes listing has one.
    #[serde(default)]
    pub liked_at: Option<i64>,
    /// Album, EP, single... when the provider knows.
    #[serde(default)]
    pub kind: Option<ReleaseKind>,
    /// How the artist whose discography this came from is credited on it.
    /// Only a discography has one.
    #[serde(default)]
    pub credit: Option<Credit>,
}

impl Album {
    /// How `artist_id` is credited on this album. Without a `credit`, it is
    /// theirs unless it names someone else as its artist.
    pub fn credit_on(&self, artist_id: i64) -> Credit {
        self.credit.unwrap_or(match self.artist_id {
            Some(id) if id != artist_id => Credit::Appears,
            _ => Credit::Main,
        })
    }

    /// Just the year, which is all the list rows have room for.
    pub fn year(&self) -> String {
        self.released
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("")
            .to_string()
    }
}

/// What sort of release an album is, as the artist page groups them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseKind {
    Album,
    Ep,
    Single,
    Live,
    Compilation,
}

/// How an artist is credited on a release in their discography.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Credit {
    /// Theirs, alone or as one of several main artists.
    Main,
    /// Someone else's, with them featured.
    Featured,
    /// Credited some other way: remixer, or composer of a song covered on it.
    Appears,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Artist {
    pub id: i64,
    pub name: String,
    pub albums_count: Option<i64>,
    #[serde(default)]
    pub image: Option<String>,
    /// When it was liked, in Unix seconds. Only a likes listing has one.
    #[serde(default)]
    pub liked_at: Option<i64>,
    /// As plain text, one paragraph a line. Only `artist` sends one.
    #[serde(default)]
    pub biography: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub tracks_count: Option<i64>,
    pub owner: Option<String>,
}

/// An album and its tracklist.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AlbumTracks {
    pub album: Album,
    pub tracks: Vec<Track>,
}

/// What a search finds, of each kind.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
}

/// The releases out between `since` and `until`, both `YYYY-MM-DD` and
/// inclusive, newest first, from several artists' discographies at once.
///
/// Only an artist's own: a feature or a remix credit is someone else's
/// release. One album reached through two liked artists shows once, and so
/// does a release carried as several same-day editions (explicit and clean,
/// standard and deluxe artwork), which would otherwise fill a row.
pub fn recent_releases(albums: Vec<Album>, since: &str, until: &str) -> Vec<Album> {
    let mut out: Vec<Album> = albums
        .into_iter()
        .filter(|album| album.credit.is_none_or(|credit| credit == Credit::Main))
        .filter(|album| {
            album
                .released
                .as_deref()
                .and_then(|date| date.get(..10))
                .is_some_and(|date| (since..=until).contains(&date))
        })
        .collect();
    out.sort_by(|a, b| {
        b.released
            .cmp(&a.released)
            .then_with(|| a.title.cmp(&b.title))
    });

    let mut ids = std::collections::HashSet::new();
    let mut editions = std::collections::HashSet::new();
    out.retain(|album| {
        ids.insert(album.id.clone())
            && editions.insert((
                album.artist_id,
                album.title.to_lowercase(),
                album.released.clone(),
            ))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credit_falls_back_to_the_album_artist() {
        let album = |artist_id| Album {
            artist_id,
            ..Default::default()
        };
        assert_eq!(album(Some(1)).credit_on(1), Credit::Main);
        assert_eq!(album(None).credit_on(1), Credit::Main);
        assert_eq!(album(Some(2)).credit_on(1), Credit::Appears);
    }

    #[test]
    fn recent_releases_keep_the_artists_own_inside_the_window_newest_first() {
        let album = |id: &str, title: &str, artist_id, released: &str, credit| Album {
            id: id.into(),
            title: title.into(),
            artist_id: Some(artist_id),
            released: Some(released.into()),
            credit: Some(credit),
            ..Default::default()
        };
        let albums = vec![
            album("old", "Old", 1, "2026-01-01", Credit::Main),
            album("a", "Single", 1, "2026-09-01", Credit::Main),
            album("feat", "Featuring", 2, "2026-09-02", Credit::Featured),
            album("b", "Album", 2, "2026-10-01", Credit::Main),
            // The same album, reached through its second main artist.
            album("b", "Album", 2, "2026-10-01", Credit::Main),
            // A clean edition of the single, out the same day.
            album("a2", "single", 1, "2026-09-01", Credit::Main),
            album("future", "Pre-order", 1, "2026-12-01", Credit::Main),
            Album {
                id: "undated".into(),
                ..Default::default()
            },
        ];
        let ids: Vec<_> = recent_releases(albums, "2026-07-17", "2026-10-15")
            .into_iter()
            .map(|album| album.id)
            .collect();
        assert_eq!(ids, ["b", "a"]);
    }

    fn track(id: i64, isrc: Option<&str>) -> Track {
        Track {
            id,
            isrc: isrc.map(str::to_string),
            ..Default::default()
        }
    }

    #[test]
    fn one_recording_across_two_releases_is_one_identity() {
        assert_eq!(
            track(111, Some("GBAAA9100123")).identity(),
            track(222, Some("GBAAA9100123")).identity()
        );
        assert_eq!(
            track(111, Some("gbaaa9100123")).identity(),
            track(222, Some("GBAAA9100123")).identity()
        );
    }

    #[test]
    fn an_untagged_track_is_only_ever_a_duplicate_of_itself() {
        assert_eq!(track(111, None).identity(), track(111, None).identity());
        assert_ne!(track(111, None).identity(), track(222, None).identity());
        // A blank ISRC is none: taking it at face value would collapse every
        // untagged track into one.
        assert_eq!(
            track(111, Some("   ")).identity(),
            TrackIdentity::Entry(111)
        );
        assert_ne!(
            track(111, Some("GBAAA9100123")).identity(),
            track(111, None).identity()
        );
    }

    #[test]
    fn durations_read_as_minutes_and_seconds() {
        let track = |duration| Track {
            duration,
            ..Default::default()
        };
        assert_eq!(track(Some(245)).duration_label(), "4:05");
        assert_eq!(track(Some(0)).duration_label(), "");
        assert_eq!(track(None).duration_label(), "");
    }
}
