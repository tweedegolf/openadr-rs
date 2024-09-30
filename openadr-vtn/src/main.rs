use tokio::{net::TcpListener, signal};
use tracing::{error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cfg(feature = "postgres")]
use openadr_vtn::data_source::PostgresStorage;
use openadr_vtn::{jwt::JwtManager, state::AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer().with_file(true).with_line_number(true))
        .with(EnvFilter::from_default_env())
        .init();

    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("listening on http://{}", listener.local_addr().unwrap());

    #[cfg(feature = "postgres")]
    let storage = PostgresStorage::from_env().await.unwrap();

    #[cfg(not(feature = "postgres"))]
    compile_error!(
        "No storage backend selected. Please enable the `postgres` feature flag during compilation"
    );

    // TODO make the JWT secret secure and configurable
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
