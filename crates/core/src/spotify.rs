//! Spotify link resolver for the native playlist bridge.
//!
//! The embedded addon reads a playlist by fetching the embed page (the public
//! page is a bot shell) and walking `__NEXT_DATA__`. This is that walk, using
//! the same proxied fetch as the rest of the addon host.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpTrack {
    pub title: String,
    pub artist: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// What kind of Spotify URL `spotify_resolve` was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlKind {
    /// `/track/` and `/episode/` go through the public oEmbed endpoint.
    Oembed,
    /// Playlists and albums are read from the embed page.
    Embed,
}

/// `Only Spotify links are supported here.` when `url` is not a Spotify link.
pub fn classify_url(url: &str) -> Result<UrlKind, &'static str> {
    let u = url.trim();
    if !u.to_lowercase().contains("spotify.com") {
        return Err("Only Spotify links are supported here.");
    }
    if u.contains("/track/") || u.contains("/episode/") {
        Ok(UrlKind::Oembed)
    } else {
        Ok(UrlKind::Embed)
    }
}

fn embed_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"open\.spotify\.com/(playlist|track|album)/([A-Za-z0-9]+)").unwrap()
    })
}

fn next_data_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?s)<script id="__NEXT_DATA__" type="application/json">(.*?)</script>"#)
            .unwrap()
    })
}

/// `/playlist/<id>` → `https://open.spotify.com/embed/<type>/<id>`.
pub fn embed_url(u: &str) -> String {
    match embed_re().captures(u) {
        Some(c) => format!(
            "https://open.spotify.com/embed/{}/{}",
            c[1].to_lowercase(),
            &c[2]
        ),
        None => String::new(),
    }
}

/// Embed pages report milliseconds. Some other shapes report seconds.
/// A value above 10s in millisecond form is already > 10_000; a length in
/// seconds only crosses that once the track is about 2.8 hours.
const DURATION_ALREADY_MS: u64 = 10_000;

fn duration_ms(v: &Value) -> Option<u64> {
    if let Some(d) = v.get("duration").and_then(|x| x.as_u64()) {
        return Some(if d > DURATION_ALREADY_MS {
            d
        } else {
            d.saturating_mul(1000)
        });
    }
    v.get("duration_ms").and_then(|x| x.as_u64())
}

fn artist_name(v: &Value) -> String {
    if let Some(arr) = v.get("artists").and_then(|x| x.as_array()) {
        let names: Vec<&str> = arr
            .iter()
            .filter_map(|x| x.as_str().or_else(|| x.get("name").and_then(|n| n.as_str())))
            .filter(|s| !s.trim().is_empty())
            .collect();
        if !names.is_empty() {
            return names.join(", ");
        }
    }
    v.get("subtitle")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn is_track_like(v: &Value) -> bool {
    v.get("title")
        .and_then(|x| x.as_str())
        .is_some_and(|s| !s.trim().is_empty())
        && !artist_name(v).trim().is_empty()
}

fn has_track_evidence(v: &Value) -> bool {
    v.get("duration").and_then(|x| x.as_u64()).is_some()
        || v.get("duration_ms").and_then(|x| x.as_u64()).is_some()
        || v.get("uri")
            .and_then(|x| x.as_str())
            .is_some_and(|s| s.starts_with("spotify:track"))
}

/// `walk()` from `addon.js`: a track array is either named like a list or
/// carries per-item evidence (duration / `spotify:track` uri). A collected
/// array is not walked again, so a track is not recorded twice.
fn walk_tracks(node: &Value, key: &str, out: &mut Vec<SpTrack>) {
    match node {
        Value::Array(items) => {
            if !items.is_empty() && items.iter().all(is_track_like) {
                let named = matches!(
                    key.to_lowercase().as_str(),
                    "tracklist" | "tracks" | "items" | "track_list"
                );
                if named || items.iter().all(has_track_evidence) {
                    for it in items {
                        let artist = artist_name(it).trim().to_string();
                        out.push(SpTrack {
                            title: it
                                .get("title")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .trim()
                                .to_string(),
                            artist: if artist.is_empty() {
                                "Unknown artist".to_string()
                            } else {
                                artist
                            },
                            duration_ms: duration_ms(it),
                        });
                    }
                    return;
                }
            }
            for it in items {
                walk_tracks(it, key, out);
            }
        }
        Value::Object(map) => {
            for (k, v) in map {
                walk_tracks(v, k, out);
            }
        }
        _ => {}
    }
}

fn next_data(html: &str) -> Option<Value> {
    let raw = next_data_re().captures(html)?.get(1)?.as_str();
    serde_json::from_str(raw).ok()
}

/// Playlist name and tracks from one embed-page body. `__NEXT_DATA__` is parsed once.
pub fn tracks_from_html(html: &str) -> (String, Vec<SpTrack>) {
    let Some(data) = next_data(html) else {
        return (String::new(), Vec::new());
    };
    let mut tracks = Vec::new();
    walk_tracks(&data, "", &mut tracks);
    let name = data
        .pointer("/props/pageProps/state/data/entity/name")
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    (name, tracks)
}

fn track_from_oembed(v: &Value) -> Option<SpTrack> {
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())?;
    let artist = v
        .get("author_name")
        .and_then(|x| x.as_str())
        .unwrap_or("Unknown artist")
        .trim()
        .to_string();
    Some(SpTrack {
        title: title.trim().to_string(),
        artist,
        duration_ms: None,
    })
}

/// Resolve a Spotify link to `(playlist name, tracks)`. Single tracks go
/// through the public oEmbed endpoint; everything else scrapes the embed page.
pub async fn spotify_resolve(
    app: crate::ctx::AppHandle,
    url: String,
) -> Result<(String, Vec<SpTrack>), String> {
    let u = url.trim().to_string();
    let kind = classify_url(&u)?;
    if kind == UrlKind::Oembed {
        let oe = format!("https://open.spotify.com/oembed?url={u}");
        if let Ok(body) = crate::addon::addon_fetch_url(&app, oe).await {
            if let Ok(v) = serde_json::from_str::<Value>(&body) {
                if let Some(track) = track_from_oembed(&v) {
                    return Ok((String::new(), vec![track]));
                }
            }
        }
    }
    let embed = embed_url(&u);
    let html = crate::addon::addon_fetch_url(&app, if embed.is_empty() { u } else { embed }).await?;
    let (name, tracks) = tracks_from_html(&html);
    if tracks.is_empty() {
        return Err("Could not read tracks from this Spotify page. Spotify may have changed its layout — use manual paste below.".to_string());
    }
    Ok((name, tracks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_non_spotify_link() {
        assert_eq!(
            classify_url("https://example.com/playlist/1"),
            Err("Only Spotify links are supported here.")
        );
    }

    #[test]
    fn classifies_track_as_oembed_and_playlist_as_embed() {
        assert_eq!(
            classify_url("https://open.spotify.com/track/abc"),
            Ok(UrlKind::Oembed)
        );
        assert_eq!(
            classify_url("https://open.spotify.com/playlist/abc"),
            Ok(UrlKind::Embed)
        );
    }

    #[test]
    fn embed_url_rewrites_the_public_path() {
        assert_eq!(
            embed_url("https://open.spotify.com/playlist/AbC123"),
            "https://open.spotify.com/embed/playlist/AbC123"
        );
        assert_eq!(embed_url("https://example.com/nope"), "");
    }

    #[test]
    fn duration_keeps_milliseconds_and_scales_seconds() {
        let ms = serde_json::json!({"duration": 180_000});
        let secs = serde_json::json!({"duration": 180});
        let field = serde_json::json!({"duration_ms": 5000});
        assert_eq!(duration_ms(&ms), Some(180_000));
        assert_eq!(duration_ms(&secs), Some(180_000));
        assert_eq!(duration_ms(&field), Some(5000));
    }

    #[test]
    fn html_yields_the_named_list_once() {
        let html = r#"
            <script id="__NEXT_DATA__" type="application/json">
            {"props":{"pageProps":{"state":{"data":{
                "entity":{"name":"Morning"},
                "widgets":[{"title":"Not a track list","artists":[{"name":"Ada"}],
                    "nested":{"tracks":[
                        {"title":"Song","artists":[{"name":"Ada"},{"name":"Bea"}],"duration":180000,"uri":"spotify:track:abc"},
                        {"title":"Other","artists":["Cy"],"duration":180}
                    ]}}]
            }}}}}
            </script>
        "#;
        let (name, tracks) = tracks_from_html(html);
        assert_eq!(name, "Morning");
        assert_eq!(
            tracks,
            vec![
                SpTrack {
                    title: "Song".into(),
                    artist: "Ada, Bea".into(),
                    duration_ms: Some(180_000),
                },
                SpTrack {
                    title: "Other".into(),
                    artist: "Cy".into(),
                    duration_ms: Some(180_000),
                },
            ]
        );
    }
}
