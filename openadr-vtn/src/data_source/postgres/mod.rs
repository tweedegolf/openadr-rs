use crate::data_source::postgres::event::PgEventStorage;
use crate::data_source::{AuthInfo, AuthSource, DataSource, EventCrud, ProgramCrud, ReportCrud};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

mod event;

#[derive(Clone)]
pub struct PostgresStorage {
    db: PgPool,
    pub auth: Arc<RwLock<Vec<AuthInfo>>>,
}

impl DataSource for PostgresStorage {
    fn programs(&self) -> Arc<dyn ProgramCrud> {
        todo!();
    }

    fn reports(&self) -> Arc<dyn ReportCrud> {
        todo!();
    }

    fn events(&self) -> Arc<dyn EventCrud> {
        Arc::<PgEventStorage>::new(self.db.clone().into())
    }

    fn auth(&self) -> Arc<dyn AuthSource> {
        self.auth.clone() // TODO
    }
}
