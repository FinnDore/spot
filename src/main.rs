mod spotify;

use std::{env, sync::Arc};

use axum::{
    body,
    extract::{Path, Query},
    http::{request::Parts, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::Deserialize;
use spotify::{MediaState, Spot};
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::{info, instrument, level_filters::LevelFilter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

use crate::spotify::Item;

#[tokio::main]
#[instrument]
async fn main() {
    let env = std::env::var("ENV").unwrap_or("production".into());
    if env == "development" {
        tracing_subscriber::fmt().without_time().init();
    } else {
        let env_filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env()
            .expect("Failed to create env filter invalid RUST_LOG env var");

        let registry = Registry::default().with(env_filter).with(fmt::layer());

        if let Ok(_) = std::env::var("AXIOM_TOKEN") {
            let axiom_layer = tracing_axiom::builder()
                .with_service_name("spot")
                .with_tags(&[(
                    &"deployment_id",
                    &std::env::var("RAILWAY_DEPLOYMENT_ID")
                        .map(|s| {
                            s + "-"
                                + std::env::var("RAILWAY_DEPLOYMENT_ID")
                                    .unwrap_or("unknown_replica".into())
                                    .as_str()
                        })
                        .unwrap_or("unknown_deployment".into()),
                )])
                .with_tags(&[(&"service.name", "spot".into())])
                .layer()
                .expect("Axiom layer failed to initialize");

            registry
                .with(axiom_layer)
                .try_init()
                .expect("Failed to initialize tracing with axiom");
            info!("Initialized tracing with axiom");
        } else {
            registry.try_init().expect("Failed to initialize tracing");
        }
    };

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
        .layer(CorsLayer::new().allow_origin(AllowOrigin::predicate(
            |origin: &HeaderValue, _request_parts: &Parts| {
                if let Ok(host) = origin.to_str() {
                    return [
                        "https://finndore.dev",
                        "finnnn.vercel.app",
                        "http://localhost:3000",
                    ]
                    .into_iter()
                    .any(|allowed_origin| host.ends_with(allowed_origin));
                }
                info!(?origin, "Cors layer failed to parse origin header");
                false
            },
        )))
        .layer(Extension(state))
        .layer(Extension(state_two));

    let port = std::env::var("PORT").unwrap_or("3001".to_string());
    let host = format!("0.0.0.0:{:}", port);
    info!("Running server on {:}", host);

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

#[instrument(skip(state, headers))]
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

    info!(%new_player_state, "Updating player state");
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

#[instrument(skip(state))]
async fn get_current_song(Extension(state): Extension<SharedState>) -> Response {
    let spot = &mut state.lock().await.spot;
    info!("Getting current song ",);
    match spot.get_current_song().await {
        Ok(song) => Json(song).into_response(),
        Err(_) => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(body::Empty::new())
            .unwrap()
            .into_response(),
    }
}

#[derive(Deserialize)]
struct TopSongsQuery {
    limit: Option<usize>,
}

#[instrument(skip(state, query))]
async fn get_top_songs(
    Extension(state): Extension<SharedState>,
    query: Option<Query<TopSongsQuery>>,
) -> Response {
    let limit = query.map(|q| q.limit).flatten().unwrap_or(4);
    let spot = &mut state.lock().await.spot;
    info!("Getting top songs");
    match spot.get_top_songs().await {
        Ok(songs) => Json(songs.into_iter().take(limit).collect::<Vec<Item>>()).into_response(),
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
