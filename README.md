# stelly-provider

What [Stelly](https://github.com/nab-os/stelly) asks a music provider for,
and how. A provider is a small service in front of a catalogue, a streaming
account or a folder of files, that answers the same questions whatever is
behind it: search, a track, an album and its tracklist, an artist and their
discography, and where a track's audio is. Stelly asks those, and never
learns what the provider is.

This crate is the provider protocol: the catalogue's types (`types`), what
else crosses the wire (`wire`), every call by name (`api`), and a client of
any provider, on reqwest with the `async` feature or on ureq with
`blocking`. With neither it is the types and the protocol alone, which is
what a provider itself depends on.

```toml
stelly-provider = { git = "https://github.com/nab-os/stelly-provider", tag = "v0.2.1", features = ["async"] }
```

```rust
let provider = stelly_provider::Client::new(Config::from_env(None));
let found = provider.send(api::search("Fleuves", 10)).await?;
let file = provider.at(Priority::Bulk).send(api::file_url(found.tracks[0].id, Quality::Lossy)).await?;
```

## The protocol

Everything is JSON over HTTP under `/v1`, with an optional bearer token and
an `x-priority: interactive | bulk` header for providers that ration their
requests. A search may name its search box in `x-search-session`, so a
provider that queues searches can drop one a newer search from the same box
has replaced, answering it 409; a provider that does not queue ignores it. A provider that cannot do something answers `{"error": "..."}`
with a 4xx or 5xx status.

| Call | Answers |
|---|---|
| `GET /v1/health` | `{"ok": true}`, without a token |
| `GET /v1/capabilities` | `Capabilities`: its name, qualities, and which optional calls it answers |
| `GET /v1/account` | `Account` |
| `GET /v1/search?q=&limit=` | `SearchResults` |
| `GET /v1/tracks/{id}` | `Track` |
| `GET /v1/tracks/{id}/file?quality=lossy\|lossless\|hires` | `FileUrl` |
| `GET /v1/albums/{id}` | `AlbumTracks` |
| `GET /v1/artists/{id}` | `Artist` |
| `GET /v1/artists/{id}/releases?own=&cap=` | `Page<Album>` |
| `GET /v1/artists/{id}/similar?limit=` | `Vec<Artist>`, optional |
| `GET /v1/favourites/{tracks\|albums\|artists}?cap=` | `Page<…>`, optional |
| `GET /v1/playlists?cap=` | `Page<Playlist>`, optional |
| `GET /v1/playlists/{id}/tracks?cap=` | `Page<Track>`, optional |
| `POST /v1/playlists` | `Created`, optional |
| `POST /v1/import/{track\|album\|artist}` | `Vec<Option<…>>`, optional |

Clients read `STELLY_PROVIDER_URL` (default `http://127.0.0.1:7710`) and
`STELLY_PROVIDER_TOKEN`, from the environment or a .env they name.

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
