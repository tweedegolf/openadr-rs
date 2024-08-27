use crate::data_source::postgres::event::PgEventStorage;
use crate::data_source::{AuthInfo, AuthSource, DataSource, EventCrud, ProgramCrud, ReportCrud};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

mod event;
mod program;

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

impl PostgresStorage {
    pub async fn new(db_url: &str) -> Result<Self, sqlx::Error> {
        let db = PgPool::connect(db_url).await?;

        Ok(Self {
            db,
            auth: Arc::new(Default::default()),
        })
    }
}
