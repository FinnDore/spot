use openssl::error;
use serde::{Deserialize, Serialize};

const TEN_SECONDS: i64 = 10000;
const TEN_MINUTES: i64 = TEN_SECONDS * 60;

pub struct Spot {
    pub client_id: String,
    pub client_secret: String,
    pub token: String,
    pub refresh_token: String,
    pub auth_expires_at: i64,
    pub current_song_cached_response: Option<CurrentSong>,
    pub current_song_cached_till: i64,
    pub current_song_cached_at: i64,
    pub top_songs_cached_response: Option<Vec<Item>>,
    pub top_songs_cached_till: i64,
}

impl Spot {
    pub fn new(client_id: String, client_secret: String, refresh_token: String) -> Self {
        Self {
            client_id,
            client_secret,
            token: String::new(),
            refresh_token,
            auth_expires_at: 0,
            current_song_cached_response: None,
            current_song_cached_till: 0,
            current_song_cached_at: 0,
            top_songs_cached_response: None,
            top_songs_cached_till: 0,
        }
    }

    pub async fn get_token(&mut self) -> Result<(), ()> {
        let client = reqwest::Client::new();
        let res = client
            .post("https://accounts.spotify.com/api/token")
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &self.refresh_token),
            ])
            .send()
            .await;

        if let Err(error) = &res {
            println!("Could not get users token ${:#?}", error);
            return Err(());
        }

        let body = res.unwrap().text().await;

        if let Err(err) = &body {
            println!("Could not decode spotify body {:?}", err);
            return Err(());
        }

        let json = serde_json::from_str(&body.unwrap());

        if let Err(err) = &json {
            println!("Could not parse spotify response to json {:?}", err);
            return Err(());
        }

        let json: AuthResponse = json.unwrap();
        self.token = json.access_token;
        self.auth_expires_at = json.expires_in + chrono::Utc::now().timestamp();

        println!("Updated spotify token");
        Ok(())
    }

    pub async fn get_current_song(&mut self) -> Result<CurrentSong, ()> {
        if chrono::Utc::now().timestamp_millis() < self.current_song_cached_till
            && self.current_song_cached_response.is_some()
        {
            let mut current_song = self.current_song_cached_response.clone().unwrap();
            current_song.progress_ms +=
                chrono::Utc::now().timestamp_millis() - self.current_song_cached_at;

            return Ok(current_song);
        } else if chrono::Utc::now().timestamp_millis() < self.current_song_cached_till {
            return Err(());
        }

        if chrono::Utc::now().timestamp() > self.auth_expires_at {
            if let Err(_) = self.get_token().await {
                return Err(());
            }
        }

        let client = reqwest::Client::new();
        let res = client
            .get("https://api.spotify.com/v1/me/player/currently-playing")
            .header("authorization", format!("Bearer {:}", self.token))
            .send()
            .await;

        let mut errored = false;
        if let Err(error) = &res {
            println!("Could not get current song ${:#?}", error);
            errored = true;
        }

        let response = res.unwrap();
        if !response.status().is_success() {
            println!("Could not get current song ${:#?}", response);
            errored = true;
        }

        if response.status() == 204 {
            // No song playing
            errored = true;
        }

        let body = response.text().await;
        if let Err(err) = &body {
            println!("Could not decode spotify body {:?}", err);
            errored = true;
        }

        let json = serde_json::from_str(&body.unwrap());

        if let Err(err) = &json {
            println!("Could not parse spotify response to json {:?}", err);
            errored = true;
        }

        if errored {
            self.current_song_cached_response = None;
            self.current_song_cached_till = chrono::Utc::now().timestamp_millis() + TEN_SECONDS;
            self.current_song_cached_at = chrono::Utc::now().timestamp_millis();
            return Err(());
        }
        let response_json: CurrentSong = json.unwrap();
        self.current_song_cached_response = Some(response_json.clone());
        self.current_song_cached_till = chrono::Utc::now().timestamp_millis()
            + std::cmp::min(
                TEN_SECONDS,
                response_json.item.duration_ms - response_json.progress_ms,
            );

        self.current_song_cached_at = chrono::Utc::now().timestamp_millis();

        Ok(response_json)
    }

    pub async fn get_top_songs(&mut self) -> Result<Vec<Item>, ()> {
        if chrono::Utc::now().timestamp_millis() < self.top_songs_cached_till
            && self.top_songs_cached_response.is_some()
        {
            return Ok(self.top_songs_cached_response.clone().unwrap());
        } else if chrono::Utc::now().timestamp_millis() < self.top_songs_cached_till {
            return Err(());
        }

        if chrono::Utc::now().timestamp() > self.auth_expires_at {
            if let Err(_) = self.get_token().await {
                return Err(());
            }
        }

        let client = reqwest::Client::new();
        let res = client
            .get("https://api.spotify.com/v1/me/top/tracks?limit=4&time_range=short_term")
            .header("authorization", format!("Bearer {:}", self.token))
            .send()
            .await;

        let mut errored = false;
        if let Err(error) = &res {
            println!("Could not get current song ${:#?}", error);
            errored = true;
        }

        let response = res.unwrap();
        if !response.status().is_success() {
            println!("Could not get top song ${:#?}", response);
            errored = true;
        }

        let body = response.text().await;
        if let Err(err) = &body {
            println!("Could not decode spotify body {:?}", err);
            errored = true;
        }

        let json: Result<TopItems, serde_json::Error> = serde_json::from_str(&body.unwrap());
        if let Err(err) = &json {
            println!("Could not parse spotify response to json {:?}", err);
            errored = true;
        }

        if errored {
            self.top_songs_cached_response = None;
            self.top_songs_cached_till = chrono::Utc::now().timestamp_millis() + TEN_SECONDS * 2;
            return Err(());
        }

        let json: TopItems = json.unwrap();

        self.top_songs_cached_response = Some(json.items.clone());
        self.top_songs_cached_till = chrono::Utc::now().timestamp_millis() + TEN_MINUTES;

        return Ok(json.items);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    #[serde(rename(serialize = "accessToken"))]
    access_token: String,
    #[serde(rename(serialize = "expiresIn"))]
    expires_in: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurrentSong {
    #[serde(rename(serialize = "progressMs"))]
    progress_ms: i64,
    timestamp: i64,
    item: Item,
    is_playing: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Item {
    name: String,
    #[serde(rename(serialize = "durationMs"))]
    duration_ms: i64,
    #[serde(rename(serialize = "preview_url"))]
    preview_url: String,
    album: Album,
    artists: Vec<Artist>,
    #[serde(rename(serialize = "externalUrls"))]
    external_urls: ExternalUrls,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Album {
    #[serde(rename(serialize = "albumType"))]
    album_type: String,
    artists: Vec<Artist>,
    #[serde(rename(serialize = "externalUrls"))]
    external_urls: ExternalUrls,
    images: Vec<Image>,
    name: String,
    uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Artist {
    #[serde(rename(serialize = "externalUrls"))]
    external_urls: ExternalUrls,
    href: String,
    name: String,
    uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ExternalUrls {
    spotify: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Image {
    height: i64,
    url: String,
    width: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TopItems {
    items: Vec<Item>,
}
