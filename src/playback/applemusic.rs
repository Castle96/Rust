// Simple Apple Music adapter stub (A1)
// This is a placeholder that implements PlaybackAdapter so we can run tests and demos
// without needing an Apple Music developer account. Later this can be extended to
// perform OAuth and call the Apple Music API.

use crate::playback::PlaybackAdapter;
use anyhow::{Context, Result};

pub struct AppleMusicAdapter {
    // if enabled, will call Apple Music API using developer token
    enabled: bool,
    dev_token: Option<String>,
    user_token: Option<String>,
    client: Option<reqwest::Client>,
    // placeholder internal state for playback control
    playing: bool,
    last_item: Option<String>,
    storefront: String,
}

impl AppleMusicAdapter {
    pub fn new() -> Self {
        // Attempt to configure Apple Music if env vars are present
        let enabled = std::env::var("APPLE_MUSIC_ENABLED").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);
        let mut dev_token: Option<String> = None;
        let mut client: Option<reqwest::Client> = None;
        let user_token = std::env::var("APPLE_MUSIC_USER_TOKEN").ok();
        let storefront = std::env::var("APPLE_MUSIC_STORE").unwrap_or_else(|_| "us".into());

        if enabled {
            // If a developer token is provided via env, use it; otherwise try to generate one from key info
            if let Ok(t) = std::env::var("APPLE_MUSIC_DEVELOPER_TOKEN") {
                dev_token = Some(t);
            } else {
                // try to generate using team/key/private path
                if let (Ok(team_id), Ok(key_id), Ok(p8_path)) = (
                    std::env::var("APPLE_MUSIC_TEAM_ID"),
                    std::env::var("APPLE_MUSIC_KEY_ID"),
                    std::env::var("APPLE_MUSIC_PRIVATE_KEY_PATH"),
                ) {
                    let ttl_sec = std::env::var("APPLE_MUSIC_DEVELOPER_TOKEN_TTL_SEC").ok().and_then(|s| s.parse::<i64>().ok()).unwrap_or(60*60*24*30*3); // default ~3 months
                    match crate::playback::applemusic_oauth::generate_developer_token(&team_id, &key_id, &p8_path, ttl_sec) {
                        Ok(tok) => dev_token = Some(tok),
                        Err(e) => eprintln!("applemusic: failed to generate developer token: {}", e),
                    }
                }
            }

            if dev_token.is_some() {
                // build reqwest client
                let c = reqwest::Client::builder().build();
                match c {
                    Ok(cl) => { client = Some(cl); }
                    Err(e) => { eprintln!("applemusic: failed to build http client: {}", e); }
                }
            }
        }

        Self {
            enabled,
            dev_token,
            user_token,
            client,
            playing: false,
            last_item: None,
            storefront,
        }
    }
}

impl Default for AppleMusicAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl PlaybackAdapter for AppleMusicAdapter {
    async fn search(&mut self, query: &str) -> Result<String> {
        if !self.enabled || self.client.is_none() || self.dev_token.is_none() {
            return Ok(format!("apple-music-stub: simulated results for '{}'", query));
        }

        // perform catalog search: GET /v1/catalog/{storefront}/search?term={query}&types=songs&limit=1
        let client = self.client.as_ref().unwrap();
        let url = format!("https://api.music.apple.com/v1/catalog/{}/search", self.storefront);
        let mut req = client.get(&url).query(&[("term", query), ("types", "songs"), ("limit", "1")]);
        if let Some(ref dt) = self.dev_token {
            req = req.bearer_auth(dt);
        }
        if let Some(ref ut) = self.user_token {
            req = req.header("Music-User-Token", ut.as_str());
        }

        let resp = req.send().await.context("applemusic: search request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let s = resp.text().await.unwrap_or_default();
            anyhow::bail!("applemusic: search API returned {}: {}", status, s);
        }

        let v: serde_json::Value = resp.json::<serde_json::Value>().await.context("applemusic: invalid json")?;
        // navigate to results.songs.data[0]
        if let Some(song) = v.pointer("/results/songs/data/0") {
            let id = song.get("id").and_then(|j| j.as_str()).unwrap_or_default();
            let name = song.pointer("/attributes/name").and_then(|j| j.as_str()).unwrap_or_default();
            let artist = song.pointer("/attributes/artistName").and_then(|j| j.as_str()).unwrap_or_default();
            Ok(format!("{} - {} (id={})", artist, name, id))
        } else {
            Ok(format!("apple-music: no results for '{}'", query))
        }
    }

    async fn play(&mut self, track_id: Option<&str>) -> Result<()> {
        self.playing = true;
        if let Some(t) = track_id {
            self.last_item = Some(t.to_string());
        }
        Ok(())
    }

    async fn pause(&mut self) -> Result<()> {
        self.playing = false;
        Ok(())
    }

    async fn next(&mut self) -> Result<()> {
        // Apple Music playback must be controlled by MusicKit on client; adapter only manages queue locally
        Ok(())
    }

    async fn prev(&mut self) -> Result<()> {
        Ok(())
    }

    async fn status(&mut self) -> Result<String> {
        Ok(format!("apple-music enabled={} playing={} last_item={}", self.enabled, self.playing, self.last_item.clone().unwrap_or_default()))
    }

    async fn artist_info(&mut self, artist_id: &str) -> Result<String> {
        if !self.enabled || self.client.is_none() || self.dev_token.is_none() {
            return Ok(format!("apple-music-stub: artist info not available for '{}'", artist_id));
        }
        let client = self.client.as_ref().unwrap();
        let url = format!("https://api.music.apple.com/v1/catalog/{}/artists/{}", self.storefront, artist_id);
        let mut req = client.get(&url);
        if let Some(ref dt) = self.dev_token {
            req = req.bearer_auth(dt);
        }
        if let Some(ref ut) = self.user_token {
            req = req.header("Music-User-Token", ut.as_str());
        }
        let resp = req.send().await.context("applemusic: artist info request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let s = resp.text().await.unwrap_or_default();
            anyhow::bail!("applemusic: artist info API returned {}: {}", status, s);
        }
        let v: serde_json::Value = resp.json::<serde_json::Value>().await.context("applemusic: invalid json")?;
        // Extract some fields: name, genreNames, url, biography (if available in attributes)
        if let Some(art) = v.pointer("/data/0") {
            let name = art.pointer("/attributes/name").and_then(|j| j.as_str()).unwrap_or_default();
            let genres = art.pointer("/attributes/genreNames").and_then(|j| j.as_array()).map(|arr| arr.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
            let url = art.pointer("/attributes/website").and_then(|j| j.as_str()).unwrap_or_default();
            Ok(format!("{}\nGenres: {}\nURL: {}", name, genres, url))
        } else {
            Ok(format!("apple-music: no artist info for '{}'", artist_id))
        }
    }

    async fn artist_discography(&mut self, artist_id: &str) -> Result<String> {
        if !self.enabled || self.client.is_none() || self.dev_token.is_none() {
            return Ok(format!("apple-music-stub: discography not available for '{}'", artist_id));
        }
        let client = self.client.as_ref().unwrap();
        // Use relationships endpoint to fetch albums: /v1/catalog/{storefront}/artists/{id}/albums
        let url = format!("https://api.music.apple.com/v1/catalog/{}/artists/{}/albums", self.storefront, artist_id);
        let mut req = client.get(&url).query(&[("limit", "25")]);
        if let Some(ref dt) = self.dev_token {
            req = req.bearer_auth(dt);
        }
        if let Some(ref ut) = self.user_token {
            req = req.header("Music-User-Token", ut.as_str());
        }
        let resp = req.send().await.context("applemusic: artist albums request failed")?;
        let status = resp.status();
        if !status.is_success() {
            let s = resp.text().await.unwrap_or_default();
            anyhow::bail!("applemusic: artist albums API returned {}: {}", status, s);
        }
        let v: serde_json::Value = resp.json::<serde_json::Value>().await.context("applemusic: invalid json")?;
        // collect album titles and release dates
        if let Some(arr) = v.pointer("/data").and_then(|d| d.as_array()) {
            let mut items = Vec::new();
            for album in arr.iter() {
                let title = album.pointer("/attributes/name").and_then(|j| j.as_str()).unwrap_or_default();
                let date = album.pointer("/attributes/releaseDate").and_then(|j| j.as_str()).unwrap_or_default();
                items.push(format!("{} ({})", title, date));
            }
            Ok(items.join("\n"))
        } else {
            Ok(format!("apple-music: no albums for '{}'", artist_id))
        }
    }
}
