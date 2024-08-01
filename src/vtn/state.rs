use openadr::wire::event::EventId;
use openadr::wire::program::ProgramId;
use openadr::wire::report::ReportId;
use openadr::wire::{Event, Program, Report};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct AppState {
    pub programs: Arc<RwLock<HashMap<ProgramId, Program>>>,
    pub reports: Arc<RwLock<HashMap<ReportId, Report>>>,
    pub events: Arc<RwLock<HashMap<EventId, Event>>>,
}
