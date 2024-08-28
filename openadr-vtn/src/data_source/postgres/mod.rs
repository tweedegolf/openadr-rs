use crate::data_source::postgres::event::PgEventStorage;
use crate::data_source::postgres::program::PgProgramStorage;
use crate::data_source::postgres::report::PgReportStorage;
use crate::data_source::{AuthInfo, AuthSource, DataSource, EventCrud, ProgramCrud, ReportCrud};
use dotenvy::dotenv;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

mod event;
mod program;
mod report;

#[derive(Clone)]
pub struct PostgresStorage {
    db: PgPool,
    pub auth: Arc<RwLock<Vec<AuthInfo>>>,
}

impl DataSource for PostgresStorage {
    fn programs(&self) -> Arc<dyn ProgramCrud> {
        Arc::<PgProgramStorage>::new(self.db.clone().into())
    }

    fn reports(&self) -> Arc<dyn ReportCrud> {
        Arc::<PgReportStorage>::new(self.db.clone().into())
    }

    fn events(&self) -> Arc<dyn EventCrud> {
        Arc::<PgEventStorage>::new(self.db.clone().into())
    }

    fn auth(&self) -> Arc<dyn AuthSource> {
        self.auth.clone() // TODO
    }
}

impl PostgresStorage {
    pub fn new(db: PgPool) -> Result<Self, sqlx::Error> {
        Ok(Self {
            db,
            auth: Arc::new(Default::default()),
        })
    }

    pub async fn from_env() -> Result<Self, sqlx::Error> {
        dotenv().unwrap();
        let db_url = std::env::var("DATABASE_URL")
            .expect("Missing DATABASE_URL env var even though the 'postgres' feature is active");

        let db = PgPool::connect(&db_url).await?;

        let connect_options = db.connect_options();
        let safe_db_url = format!(
            "{}:{}/{}",
            connect_options.get_host(),
            connect_options.get_port(),
            connect_options.get_database().unwrap_or_default()
        );

        Self::new(db)
            .inspect_err(|err| error!(?err, "could not connect to Postgres database"))
            .inspect(|_| {
                info!(
                    "Successfully connected to Postgres backend at {}",
                    safe_db_url
                )
            })
    }
}
