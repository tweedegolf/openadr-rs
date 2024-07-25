use openadr::wire::program::ProgramId;
use openadr::wire::report::ReportId;
use openadr::wire::{Program, Report};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub programs: Arc<RwLock<HashMap<ProgramId, Program>>>,
    pub reports: Arc<RwLock<HashMap<ReportId, Report>>>,
    pub pool: PgPool,
}
