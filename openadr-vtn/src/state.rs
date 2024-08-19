use axum::extract::FromRef;
use std::sync::Arc;

use crate::data_source::{AuthSource, DataSource, EventCrud, ProgramCrud, ReportCrud};
use crate::jwt::JwtManager;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub storage: Arc<dyn DataSource>,
    pub jwt_manager: Arc<JwtManager>,
}

impl AppState {
    pub fn new<S: DataSource>(storage: S, jwt_manager: JwtManager) -> Self {
        Self {
            storage: Arc::new(storage),
            jwt_manager: Arc::new(jwt_manager),
        }
    }

    fn router_without_state() -> axum::Router<Self> {
        use axum::routing::{get, post};
        use tower_http::trace::TraceLayer;

        use crate::api::program;
        use crate::api::report;
        use crate::api::{auth, event};

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

    pub fn into_router(self) -> axum::Router {
        Self::router_without_state().with_state(self)
    }
}

impl FromRef<AppState> for Arc<dyn AuthSource> {
    fn from_ref(state: &AppState) -> Arc<dyn AuthSource> {
        state.storage.auth()
    }
}

impl FromRef<AppState> for Arc<dyn ProgramCrud> {
    fn from_ref(state: &AppState) -> Arc<dyn ProgramCrud> {
        state.storage.programs()
    }
}

impl FromRef<AppState> for Arc<dyn EventCrud> {
    fn from_ref(state: &AppState) -> Arc<dyn EventCrud> {
        state.storage.events()
    }
}

impl FromRef<AppState> for Arc<dyn ReportCrud> {
    fn from_ref(state: &AppState) -> Arc<dyn ReportCrud> {
        state.storage.reports()
    }
}
