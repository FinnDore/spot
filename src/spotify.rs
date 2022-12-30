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

        let json: serde_json::Value = json.unwrap();
        self.token = json["access_token"].to_string().replace("\"", "");
        self.expires_at = json["expires_in"].as_i64().unwrap() + chrono::Utc::now().timestamp();
        println!("Updated spotify token");
        Ok(())
    }

    pub async fn get_current_song(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        if chrono::Utc::now().timestamp() > self.expires_at {
            if let Err(_) = self.get_token().await {
                return Ok("No song playing".to_string());
            }
        }

        let client = reqwest::Client::new();
        let res = client
            .get("https://api.spotify.com/v1/me/player/currently-playing")
            .header("authorization", format!("Bearer {:}", self.token))
            .send()
            .await;
        println!("Bearer {:?}", self.token);
        if let Err(error) = &res {
            println!("Could not get current song ${:#?}", error);
            return Ok("No song playing".to_string());
        }

        let response = res.unwrap();
        if !response.status().is_success() {
            println!("Could not get current song ${:#?}", response);
            return Ok("No song playing".to_string());
        }

        let body = response.text().await;

        if let Err(err) = &body {
            println!("Could not decode spotify body {:?}", err);
            return Ok("No song playing".to_string());
        }

        let json: serde_json::Value = serde_json::from_str(&body.unwrap())?;
        println!("{:#?}", json);
        let song = json["item"]["name"].to_string();
        Ok(song)
    }
}
