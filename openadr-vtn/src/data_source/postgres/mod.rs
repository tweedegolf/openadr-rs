use crate::data_source::postgres::event::PgEventStorage;
use crate::data_source::postgres::program::PgProgramStorage;
use crate::data_source::postgres::report::PgReportStorage;
use crate::data_source::postgres::user::PgAuthSource;
use crate::data_source::{AuthSource, DataSource, EventCrud, ProgramCrud, ReportCrud};
use crate::error::AppError;
use crate::jwt::Claims;
use dotenvy::dotenv;
use openadr_wire::target::TargetLabel;
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info};

mod event;
mod program;
mod report;
mod user;

#[derive(Clone)]
pub struct PostgresStorage {
    db: PgPool,
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
        Arc::<PgAuthSource>::new(self.db.clone().into())
    }
}

impl PostgresStorage {
    pub fn new(db: PgPool) -> Result<Self, sqlx::Error> {
        Ok(Self { db })
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

fn to_json_value<T: Serialize>(v: Option<T>) -> Result<Option<serde_json::Value>, AppError> {
    v.map(|v| serde_json::to_value(v).map_err(AppError::SerdeJsonBadRequest))
        .transpose()
}

#[derive(Serialize, Debug)]
struct PgTargetsFilter<'a> {
    #[serde(rename = "type")]
    label: &'a str,
    #[serde(rename = "values")]
    value: [String; 1],
}

#[derive(Debug)]
struct PgPermissionFilter<'a> {
    ven_ids: Vec<PgTargetsFilter<'a>>,
}

impl From<&Claims> for PgPermissionFilter<'_> {
    fn from(claims: &Claims) -> Self {
        Self {
            ven_ids: claims
                .ven_ids()
                .into_iter()
                .map(|ven_id| PgTargetsFilter {
                    label: TargetLabel::VENName.as_str(),
                    value: [ven_id],
                })
                .collect(),
        }
    }
}
