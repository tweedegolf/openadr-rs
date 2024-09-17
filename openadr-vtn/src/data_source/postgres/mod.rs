use crate::{
    data_source::{
        postgres::{
            event::PgEventStorage, program::PgProgramStorage, report::PgReportStorage,
            user::PgAuthSource, ven::PgVenStorage,
        },
        AuthSource, DataSource, EventCrud, ProgramCrud, ReportCrud, ResourceCrud, VenCrud,
    },
    error::AppError,
    jwt::{BusinessIds, Claims},
};
use dotenvy::dotenv;
use openadr_wire::target::{TargetLabel, TargetMap};
use resource::PgResourceStorage;
use serde::Serialize;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info, trace};

mod event;
mod program;
mod report;
mod resource;
mod user;
mod ven;

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

    fn vens(&self) -> Arc<dyn VenCrud> {
        Arc::<PgVenStorage>::new(self.db.clone().into())
    }

    fn resources(&self) -> Arc<dyn ResourceCrud> {
        Arc::<PgResourceStorage>::new(self.db.clone().into())
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

#[tracing::instrument(level = "trace")]
fn extract_vens(targets: Option<TargetMap>) -> (Option<TargetMap>, Option<Vec<String>>) {
    if let Some(TargetMap(targets)) = targets {
        let (vens, targets): (Vec<_>, Vec<_>) = targets
            .into_iter()
            .partition(|t| t.label == TargetLabel::VENName);

        let vens = vens
            .into_iter()
            .map(|t| t.values[0].clone())
            .collect::<Vec<_>>();

        let targets = if targets.is_empty() {
            None
        } else {
            Some(TargetMap(targets))
        };
        let vens = if vens.is_empty() { None } else { Some(vens) };

        trace!(?targets, ?vens);
        (targets, vens)
    } else {
        (None, None)
    }
}

fn extract_business_id(user: &Claims) -> Result<Option<String>, AppError> {
    match user.business_ids() {
        BusinessIds::Specific(ids) => {
            if ids.len() == 1 {
                Ok(Some(ids[0].clone()))
            } else {
                Err(AppError::BadRequest("Cannot infer business id from user"))?
            }
        }
        BusinessIds::Any => Ok(None),
    }
}

fn extract_business_ids(user: &Claims) -> Option<Vec<String>> {
    match user.business_ids() {
        BusinessIds::Specific(ids) => Some(ids),
        BusinessIds::Any => None,
    }
}

#[derive(Debug)]
struct PgId {
    id: String,
}
