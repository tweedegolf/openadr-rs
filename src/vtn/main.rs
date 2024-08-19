use axum::routing::{get, post};
use axum::Router;
use data_source::{AuthInfo, InMemoryStorage};
use jwt::{AuthRole, JwtManager};
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

use api::program;
use api::report;
use api::{auth, event};

use crate::state::AppState;

mod api;
mod data_source;
mod error;
mod jwt;
mod state;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("listening on http://{}", listener.local_addr().unwrap());

    let storage = InMemoryStorage::default();
    storage.auth.write().await.push(AuthInfo {
        client_id: "admin".to_string(),
        client_secret: "admin".to_string(),
        role: AuthRole::BL,
        ven: None,
    });
    let state = AppState::new(storage, JwtManager::from_base64_secret("test").unwrap());

    if let Err(e) = axum::serve(listener, state.into_router())
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        error!("webserver crashed: {}", e);
    }
}

impl AppState {
    fn router_without_state() -> axum::Router<Self> {
        axum::Router::new()
            .route("/programs", get(program::get_all).post(program::add))
            .route(
                "/programs/:id",
                get(program::get).put(program::edit).delete(program::delete),
            )
            .route("/reports", get(report::get_all).post(report::add))
            .route(
                "/reports/:id",
                get(report::get).put(report::edit).delete(report::delete),
            )
            .route("/events", get(event::get_all).post(event::add))
            .route(
                "/events/:id",
                get(event::get).put(event::edit).delete(event::delete),
            )
            .route("/auth/register", post(auth::register))
            .route("/auth/token", post(auth::token))
            .layer(TraceLayer::new_for_http())
    }

    pub fn into_router(self) -> Router {
        Self::router_without_state().with_state(self)
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
