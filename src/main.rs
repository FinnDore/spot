mod spotify;

use std::{env, sync::Arc};

use axum::{
    body,
    extract::Path,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use spotify::{MediaState, Spot};
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(State {
        spot: Spot::new(
            env::var("SPOTIFY_CLIENT_ID").expect("Expected SPOTIFY_CLIENT_ID env var"),
            env::var("SPOTIFY_CLIENT_SECRET").expect("Expected SPOTIFY_CLIENT_SECRET env var"),
            env::var("SPOTIFY_REFRESH_TOKEN").expect("Expected SPOTIFY_REFRESH_TOKEN env var"),
        ),
        token: env::var("EXTERNAL_AUTH_TOKEN").expect("Expected EXTERNAL_AUTH_TOKEN env var"),
    }));

    let state_two = state.clone();
    let app = Router::new()
        .route("/", get(get_current_song))
        .route("/top-songs", get(get_top_songs))
        .route("/player/:player_state", post(update_player_state))
        .layer(Extension(state))
        .layer(Extension(state_two));

    let port = std::env::var("PORT").unwrap_or("3001".to_string());
    let host = format!("0.0.0.0:{:}", port);
    println!("Running server on {:}", host);

    axum::Server::bind(&host.to_string().parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

struct State {
    spot: Spot,
    token: String,
}

type SharedState = Arc<Mutex<State>>;

async fn update_player_state(
    Path(new_player_state): Path<MediaState>,
    Extension(state): Extension<SharedState>,
    headers: HeaderMap,
) -> Response {
    let state = &mut state.lock().await;
    let incoming_token = headers.get("Authorization");
    if incoming_token.is_none() || incoming_token.unwrap() != &state.token {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(body::Empty::new())
            .unwrap()
            .into_response();
    }

    println!(
        "Updating player state time {}",
        chrono::Utc::now().to_rfc2822()
    );
    match state.spot.update_player_state(new_player_state).await {
        Ok(_) => Response::builder()
            .status(StatusCode::OK)
            .body(body::Empty::new())
            .unwrap()
            .into_response(),
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body::Empty::new())
            .unwrap()
            .into_response(),
    }
}

async fn get_current_song(Extension(state): Extension<SharedState>) -> Response {
    let spot = &mut state.lock().await.spot;
    println!(
        "Getting current song time {}",
        chrono::Utc::now().to_rfc2822()
    );
    match spot.get_current_song().await {
        Ok(song) => Json(song).into_response(),
        Err(_) => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(body::Empty::new())
            .unwrap()
            .into_response(),
    }
}

async fn get_top_songs(Extension(state): Extension<SharedState>) -> Response {
    let spot = &mut state.lock().await.spot;
    println!("Getting top songs time {}", chrono::Utc::now().to_rfc2822());
    match spot.get_top_songs().await {
        Ok(songs) => Json(songs).into_response(),
        Err(_) => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(body::Empty::new())
            .unwrap()
            .into_response(),
    }
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
