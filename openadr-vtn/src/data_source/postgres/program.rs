use crate::api::program::QueryParams;
use crate::data_source::{Crud, ProgramCrud};
use crate::error::AppError;
use axum::async_trait;
use chrono::{DateTime, Utc};
use openadr_wire::program::{ProgramContent, ProgramId};
use openadr_wire::Program;
use sqlx::PgPool;
use tracing::error;

#[async_trait]
impl ProgramCrud for PgProgramStorage {}

pub(crate) struct PgProgramStorage {
    db: PgPool,
}

impl From<PgPool> for PgProgramStorage {
    fn from(db: PgPool) -> Self {
        Self { db }
    }
}

#[derive(Debug)]
struct PostgresProgram {
    id: String,
    created_date_time: DateTime<Utc>,
    modification_date_time: DateTime<Utc>,
    program_name: String,
    program_long_name: Option<String>,
    retailer_name: Option<String>,
    retailer_long_name: Option<String>,
    program_type: Option<String>,
    country: Option<String>,
    principal_subdivision: Option<String>,
    interval_period: Option<serde_json::Value>,
    program_descriptions: Option<serde_json::Value>,
    binding_events: Option<bool>,
    local_price: Option<bool>,
    payload_descriptors: Option<serde_json::Value>,
    targets: Option<serde_json::Value>,
}

impl TryFrom<PostgresProgram> for Program {
    type Error = AppError;

    #[tracing::instrument(name = "TryFrom<PostgresProgram> for Program")]
    fn try_from(value: PostgresProgram) -> Result<Self, Self::Error> {
        let interval_period = match value.interval_period {
            None => None,
            Some(t) => serde_json::from_value(t)
                .inspect_err(|err| {
                    error!(
                        ?err,
                        "Failed to deserialize JSON from DB to `IntervalPeriod`"
                    )
                })
                .map_err(AppError::SerdeJsonInternalServerError)?,
        };
        let program_descriptions = match value.program_descriptions {
            None => None,
            Some(t) => serde_json::from_value(t)
                .inspect_err(|err| {
                    error!(
                        ?err,
                        "Failed to deserialize JSON from DB to `Vec<ProgramDescription>`"
                    )
                })
                .map_err(AppError::SerdeJsonInternalServerError)?,
        };
        let payload_descriptors = match value.payload_descriptors {
            None => None,
            Some(t) => serde_json::from_value(t)
                .inspect_err(|err| {
                    error!(
                        ?err,
                        "Failed to deserialize JSON from DB to `Vec<PayloadDescriptor>`"
                    )
                })
                .map_err(AppError::SerdeJsonInternalServerError)?,
        };
        let targets = match value.targets {
            None => None,
            Some(t) => serde_json::from_value(t)
                .inspect_err(|err| {
                    error!(?err, "Failed to deserialize JSON from DB to `TargetMap`")
                })
                .map_err(AppError::SerdeJsonInternalServerError)?,
        };

        Ok(Self {
            id: value.id.parse()?,
            created_date_time: value.created_date_time,
            modification_date_time: value.modification_date_time,
            content: ProgramContent {
                object_type: Default::default(),
                program_name: value.program_name,
                program_long_name: value.program_long_name,
                retailer_name: value.retailer_name,
                retailer_long_name: value.retailer_long_name,
                program_type: value.program_type,
                country: value.country,
                principal_subdivision: value.principal_subdivision,
                time_zone_offset: None,
                interval_period,
                program_descriptions,
                binding_events: value.binding_events,
                local_price: value.local_price,
                payload_descriptors,
                targets,
            },
        })
    }
}

#[async_trait]
impl Crud for PgProgramStorage {
    type Type = Program;
    type Id = ProgramId;
    type NewType = ProgramContent;
    type Error = AppError;
    type Filter = QueryParams;

    async fn create(&self, new: Self::NewType) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresProgram,
            r#"
            INSERT INTO program (id, created_date_time, modification_date_time, program_name, program_long_name, retailer_name, retailer_long_name, program_type, country, principal_subdivision, interval_period, program_descriptions, binding_events, local_price, payload_descriptors, targets)
            VALUES (gen_random_uuid(), now(), now(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
            new.program_name,
            new.program_long_name,
            new.retailer_name,
            new.retailer_long_name,
            new.program_type,
            new.country,
            new.principal_subdivision,
            serde_json::to_value(new.interval_period).map_err(AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(new.program_descriptions).map_err(AppError::SerdeJsonBadRequest)?,
            new.binding_events,
            new.local_price,
            serde_json::to_value(new.payload_descriptors).map_err(AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(new.targets).map_err(AppError::SerdeJsonBadRequest)?,
        )
            .fetch_one(&self.db)
            .await?
            .try_into()?)
    }

    async fn retrieve(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresProgram,
            r#"
            SELECT * FROM program WHERE id = $1
            "#,
            id.as_str()
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?)
    }

    async fn retrieve_all(&self, filter: &Self::Filter) -> Result<Vec<Self::Type>, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresProgram,
            r#"
            SELECT * FROM program
            WHERE ($1::text IS NULL OR jsonb_path_query_array(targets, '$[*].type') ? $1)
              AND ($2::text[] IS NULL OR jsonb_path_query_array(targets, '$[*].values[*]') ?| ($2))
            OFFSET $3 LIMIT $4
            "#,
            filter.target_type.clone().map(|t| t.to_string()),
            filter.target_values.as_ref().map(|v| v.as_slice()),
            filter.skip,
            filter.limit
        )
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<_, _>>()?)
    }

    async fn update(&self, id: &Self::Id, new: Self::NewType) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresProgram,
            r#"
            UPDATE program
            SET modification_date_time = now(),
                program_name = $2,
                program_long_name = $3,
                retailer_name = $4,
                retailer_long_name = $5,
                program_type = $6,
                country = $7,
                principal_subdivision = $8,
                interval_period = $9,
                program_descriptions = $10,
                binding_events = $11,
                local_price = $12,
                payload_descriptors = $13,
                targets = $14
            WHERE id = $1
            RETURNING *
            "#,
            id.as_str(),
            new.program_name,
            new.program_long_name,
            new.retailer_name,
            new.retailer_long_name,
            new.program_type,
            new.country,
            new.principal_subdivision,
            serde_json::to_value(new.interval_period).map_err(AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(new.program_descriptions)
                .map_err(AppError::SerdeJsonBadRequest)?,
            new.binding_events,
            new.local_price,
            serde_json::to_value(new.payload_descriptors).map_err(AppError::SerdeJsonBadRequest)?,
            serde_json::to_value(new.targets).map_err(AppError::SerdeJsonBadRequest)?,
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?)
    }

    async fn delete(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresProgram,
            r#"
            DELETE FROM program WHERE id = $1 RETURNING *
            "#,
            id.as_str()
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?)
    }
}
