mod spotify;

use std::{env, sync::Arc};

use axum::{
    body,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Json, Router,
};

use spotify::Spot;

use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(State {
        spot: Spot::new(
            env::var("SPOTIFY_CLIENT_ID").expect("Expected SPOTIFY_CLIENT_ID env var"),
            env::var("SPOTIFY_CLIENT_SECRET").expect("Expected SPOTIFY_CLIENT_SECRET env var"),
            env::var("SPOTIFY_REFRESH_TOKEN").expect("Expected SPOTIFY_REFRESH_TOKEN env var"),
        ),
    }));

    let state_two = state.clone();
    let app = Router::new()
        .route("/current_song", get(get_current_song))
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
}

type SharedState = Arc<Mutex<State>>;

async fn get_current_song(Extension(state): Extension<SharedState>) -> Response {
    let spot = &mut state.lock().await.spot;
    match spot.get_current_song().await {
        Ok(song) => Json(song).into_response(),
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
