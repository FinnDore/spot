use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

const SPOTIFY_URL: &str = "https://api.spotify.com/v1";
async fn get_current_song_handler() -> Response {
    if let Some(current_song) = get_current_song().await {
        return Json(current_song).into_response();
    } else {
        Json(CurrentSong {
            progress: 0,
            item: Item {
                name: "No song playing".to_string(),
            },
        })
        .into_response()
    }
}

#[tokio::main]
async fn main() {
    let current_song = get_current_song().await;
    println!("{:#?}", current_song.unwrap());

    // build our application with a single route
    let app = Router::new().route("/current_song", get(get_current_song_handler));
    let port = std::env::var("PORT").unwrap_or("3001".to_string());
    let host = format!("0.0.0.0:{:}", port);
    println!("Running server on {:}", host);
    // run it with hyper on localhost:3000
    axum::Server::bind(&host.to_string().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_current_song() -> Option<CurrentSong> {
    let client = reqwest::Client::new();
    let token = std::env::var("SPOTIFY_TOKEN").expect("Expected SPOTIFY_TOKEN env var");
    let res = client
        .get(format!("{:}/me/player/currently-playing", SPOTIFY_URL))
        .header("authorization", format!("Bearer {:}", token))
        .send()
        .await;

    if let Err(error) = &res {
        println!("Could not get current song ${:#?}", error);
        return None;
    }
    let result = res.unwrap();
    let body = result.text().await;

    if let Err(err) = &body {
        println!("Could not decode spotfiy body {:?}", err);
        return None;
    }

    let current_info: Result<CurrentSong, serde_json::Error> = serde_json::from_str(&body.unwrap());

    if let Err(err) = &current_info {
        println!(
            "Could not parse spotify response to json {:?} err: {:?}",
            current_info, err
        );
        return None;
    }

    Some(current_info.unwrap())
}

#[derive(Serialize, Deserialize, Debug)]
struct CurrentSong {
    #[serde(rename = "progress_ms")]
    progress: u128,
    item: Item,
}

#[derive(Serialize, Deserialize, Debug)]
struct Item {
    name: String,
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
