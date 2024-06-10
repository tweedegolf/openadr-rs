use std::sync::Arc;

use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

use api::event;
use api::program;
use api::report;

use crate::state::AppState;

mod api;
mod error;
mod state;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let state = AppState {
        programs: Arc::new(Default::default()),
        reports: Arc::new(Default::default()),
        events: Arc::new(Default::default()),
    };

    let app = Router::new()
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
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("listening on http://{}", listener.local_addr().unwrap());
    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        error!("webserver crashed: {}", e);
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
