use crate::data_source::{AuthInfo, AuthSource, DataSource, EventCrud, ProgramCrud, ReportCrud};
use openadr_wire::event::EventId;
use openadr_wire::program::ProgramId;
use openadr_wire::report::ReportId;
use openadr_wire::{Event, Program, Report};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

mod event;
mod program;
mod report;

#[derive(Default, Clone)]
pub struct InMemoryStorage {
    pub programs: Arc<RwLock<HashMap<ProgramId, Program>>>,
    pub reports: Arc<RwLock<HashMap<ReportId, Report>>>,
    pub events: Arc<RwLock<HashMap<EventId, Event>>>,
    pub auth: Arc<RwLock<Vec<AuthInfo>>>,
}

impl DataSource for InMemoryStorage {
    fn programs(&self) -> Arc<dyn ProgramCrud> {
        self.programs.clone()
    }

    fn reports(&self) -> Arc<dyn ReportCrud> {
        self.reports.clone()
    }

    fn events(&self) -> Arc<dyn EventCrud> {
        self.events.clone()
    }

    fn auth(&self) -> Arc<dyn AuthSource> {
        self.auth.clone()
    }
}
