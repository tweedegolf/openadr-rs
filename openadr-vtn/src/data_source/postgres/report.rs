use crate::api::report::QueryParams;
use crate::data_source::postgres::{to_json_value, PgId};
use crate::data_source::{Crud, ReportCrud};
use crate::error::AppError;
use crate::jwt::Claims;
use axum::async_trait;
use chrono::{DateTime, Utc};
use openadr_wire::report::{ReportContent, ReportId};
use openadr_wire::Report;
use sqlx::PgPool;
use tracing::error;

#[async_trait]
impl ReportCrud for PgReportStorage {}

pub(crate) struct PgReportStorage {
    db: PgPool,
}
impl From<PgPool> for PgReportStorage {
    fn from(db: PgPool) -> Self {
        Self { db }
    }
}

#[derive(Debug)]
struct PostgresReport {
    id: String,
    created_date_time: DateTime<Utc>,
    modification_date_time: DateTime<Utc>,
    program_id: String,
    event_id: String,
    client_name: String,
    report_name: Option<String>,
    payload_descriptors: Option<serde_json::Value>,
    resources: serde_json::Value,
}

impl TryFrom<PostgresReport> for Report {
    type Error = AppError;

    #[tracing::instrument(name = "TryFrom<PostgresReport> for Report")]
    fn try_from(value: PostgresReport) -> Result<Self, Self::Error> {
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
        let resources = serde_json::from_value(value.resources)
            .inspect_err(|err| error!(?err, "Failed to deserialize JSON from DB to `TargetMap`"))
            .map_err(AppError::SerdeJsonInternalServerError)?;

        Ok(Self {
            id: value.id.parse()?,
            created_date_time: value.created_date_time,
            modification_date_time: value.modification_date_time,
            content: ReportContent {
                object_type: Default::default(),
                program_id: value.program_id.parse()?,
                event_id: value.event_id.parse()?,
                client_name: value.client_name,
                report_name: value.report_name,
                payload_descriptors,
                resources,
            },
        })
    }
}

#[async_trait]
impl Crud for PgReportStorage {
    type Type = Report;
    type Id = ReportId;
    type NewType = ReportContent;
    type Error = AppError;
    type Filter = QueryParams;
    type PermissionFilter = Claims;

    async fn create(
        &self,
        new: Self::NewType,
        _user: &Self::PermissionFilter,
    ) -> Result<Self::Type, Self::Error> {
        let PgId { id } = sqlx::query_as!(
            PgId,
            r#"
            SELECT program_id AS id FROM event WHERE id = $1
            "#,
            new.event_id.as_str(),
        )
        .fetch_one(&self.db)
        .await?;

        if id != new.program_id.as_str() {
            return Err(AppError::BadRequest(
                "event_id and program_id have to point to the same program",
            ));
        }

        Ok(sqlx::query_as!(
            PostgresReport,
            r#"
            INSERT INTO report (id, created_date_time, modification_date_time, program_id, event_id, client_name, report_name, payload_descriptors, resources)
            VALUES (gen_random_uuid(), now(), now(), $1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
            new.program_id.as_str(),
            new.event_id.as_str(),
            new.client_name,
            new.report_name,
            to_json_value(new.payload_descriptors)?,
            serde_json::to_value(new.resources).map_err(AppError::SerdeJsonBadRequest)?,
        )
            .fetch_one(&self.db)
            .await?
            .try_into()?)
    }

    async fn retrieve(
        &self,
        id: &Self::Id,
        _user: &Self::PermissionFilter,
    ) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresReport,
            r#"
            SELECT * FROM report WHERE id = $1
            "#,
            id.as_str()
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?)
    }

    async fn retrieve_all(
        &self,
        filter: &Self::Filter,
        _user: &Self::PermissionFilter,
    ) -> Result<Vec<Self::Type>, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresReport,
            r#"
            SELECT * FROM report
            WHERE ($1::text IS NULL OR $1 like program_id)
              AND ($2::text IS NULL OR $2 like event_id)
              AND ($3::text IS NULL OR $3 like client_name)
            LIMIT $4 OFFSET $5
            "#,
            filter.program_id.clone().map(|x| x.to_string()),
            filter.event_id.clone().map(|x| x.to_string()),
            filter.client_name,
            filter.skip,
            filter.limit,
        )
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<_, _>>()?)
    }

    async fn update(
        &self,
        id: &Self::Id,
        new: Self::NewType,
        _user: &Self::PermissionFilter,
    ) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresReport,
            r#"
            UPDATE report
            SET modification_date_time = now(),
                program_id = $2,
                event_id = $3,
                client_name = $4,
                report_name = $5,
                payload_descriptors = $6,
                resources = $7
            WHERE id = $1
            RETURNING *
            "#,
            id.as_str(),
            new.program_id.as_str(),
            new.event_id.as_str(),
            new.client_name,
            new.report_name,
            to_json_value(new.payload_descriptors)?,
            serde_json::to_value(new.resources).map_err(AppError::SerdeJsonBadRequest)?,
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?)
    }

    async fn delete(
        &self,
        id: &Self::Id,
        _user: &Self::PermissionFilter,
    ) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresReport,
            r#"
            DELETE FROM report WHERE id = $1 RETURNING *
            "#,
            id.as_str()
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?)
    }
}
