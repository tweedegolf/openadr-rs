use tokio::net::TcpListener;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

use openadr_vtn::data_source::{AuthInfo, InMemoryStorage};
use openadr_vtn::jwt::{AuthRole, JwtManager};
use openadr_vtn::state::AppState;

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
