use serde::{Deserialize, Serialize};

pub struct Spot {
    pub client_id: String,
    pub client_secret: String,
    pub token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

impl Spot {
    pub fn new(client_id: String, client_secret: String, refresh_token: String) -> Self {
        Self {
            client_id,
            client_secret,
            token: String::new(),
            refresh_token,
            expires_at: 0,
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
        self.expires_at = json.expires_in + chrono::Utc::now().timestamp();

        println!("Updated spotify token");
        Ok(())
    }

    pub async fn get_current_song(&mut self) -> Result<CurrentSong, ()> {
        if chrono::Utc::now().timestamp() > self.expires_at {
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

        if let Err(error) = &res {
            println!("Could not get current song ${:#?}", error);
            return Err(());
        }

        let response = res.unwrap();
        if !response.status().is_success() {
            println!("Could not get current song ${:#?}", response);
            return Err(());
        }

        if response.status() == 204 {
            // No song playing
            return Err(());
        }

        let body = response.text().await;
        if let Err(err) = &body {
            println!("Could not decode spotify body {:?}", err);
            return Err(());
        }

        let json = serde_json::from_str(&body.unwrap());

        if let Err(err) = &json {
            println!("Could not parse spotify response to json {:?}", err);
            return Err(());
        }

        Ok(json.unwrap())
    }

    pub async fn get_top_songs(&mut self) -> Result<Vec<Item>, ()> {
        if chrono::Utc::now().timestamp() > self.expires_at {
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

        if let Err(error) = &res {
            println!("Could not get current song ${:#?}", error);
            return Err(());
        }

        let response = res.unwrap();
        if !response.status().is_success() {
            println!("Could not get current song ${:#?}", response);
            return Err(());
        }

        if response.status() == 204 {
            // No song playing
            return Err(());
        }

        let body = response.text().await;
        if let Err(err) = &body {
            println!("Could not decode spotify body {:?}", err);
            return Err(());
        }

        let json: Result<TopItems, serde_json::Error> = serde_json::from_str(&body.unwrap());
        if let Err(err) = &json {
            println!("Could not parse spotify response to json {:?}", err);
            return Err(());
        }

        return Ok(json.unwrap().items);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    access_token: String,
    expires_in: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CurrentSong {
    progress_ms: u128,
    timestamp: u128,
    item: Item,
    is_playing: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Item {
    name: String,
    duration_ms: u128,
    preview_url: String,
    album: Album,
    artists: Vec<Artist>,
    external_urls: ExternalUrls,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Album {
    album_type: String,
    artists: Vec<Artist>,
    external_urls: ExternalUrls,
    images: Vec<Image>,
    name: String,
    uri: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Artist {
    external_urls: ExternalUrls,
    href: String,
    name: String,
    uri: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ExternalUrls {
    spotify: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Image {
    height: u128,
    url: String,
    width: u128,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TopItems {
    items: Vec<Item>,
}
