use crate::api::ven::QueryParams;
use crate::data_source::postgres::{to_json_value, PgTargetsFilter};
use crate::data_source::{Crud, VenCrud};
use crate::error::AppError;
use crate::jwt::Claims;
use axum::async_trait;
use chrono::{DateTime, Utc};
use openadr_wire::target::TargetLabel;
use openadr_wire::ven::{Ven, VenContent, VenId};
use sqlx::PgPool;
use tracing::{error, trace};

#[async_trait]
impl VenCrud for PgVenStorage {}

pub(crate) struct PgVenStorage {
    db: PgPool,
}

impl From<PgPool> for PgVenStorage {
    fn from(db: PgPool) -> Self {
        Self { db }
    }
}

#[derive(Debug)]
struct PostgresVen {
    id: String,
    created_date_time: DateTime<Utc>,
    modification_date_time: DateTime<Utc>,
    ven_name: String,
    attributes: Option<serde_json::Value>,
    targets: Option<serde_json::Value>,
}

impl TryFrom<PostgresVen> for Ven {
    type Error = AppError;

    #[tracing::instrument(name = "TryFrom<PostgresVen> for Ven")]
    fn try_from(value: PostgresVen) -> Result<Self, Self::Error> {
        let attributes = match value.attributes {
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
            content: VenContent {
                object_type: Default::default(),
                ven_name: value.ven_name,
                targets,
                attributes,
                resources: Default::default(),
            },
        })
    }
}

#[derive(Debug, Default)]
struct PostgresFilter<'a> {
    ven_names: Option<&'a [String]>,
    resource_names: Option<&'a [String]>,
    targets: Vec<PgTargetsFilter<'a>>,
    skip: i64,
    limit: i64,
}

impl<'a> From<&'a QueryParams> for PostgresFilter<'a> {
    fn from(query: &'a QueryParams) -> Self {
        let mut filter = Self {
            skip: query.skip,
            limit: query.limit,
            ..Default::default()
        };
        match query.target_type {
            Some(TargetLabel::VENName) => filter.ven_names = query.target_values.as_deref(),
            Some(TargetLabel::ResourceName) => {
                filter.resource_names = query.target_values.as_deref()
            }
            Some(ref label) => {
                if let Some(values) = query.target_values.as_ref() {
                    filter.targets = values
                        .iter()
                        .map(|value| PgTargetsFilter {
                            label: label.as_str(),
                            value: [value.clone()],
                        })
                        .collect()
                }
            }
            None => {}
        };

        filter
    }
}

#[async_trait]
impl Crud for PgVenStorage {
    type Type = Ven;
    type Id = VenId;
    type NewType = VenContent;
    type Error = AppError;
    type Filter = QueryParams;
    type PermissionFilter = Claims;

    async fn create(
        &self,
        new: Self::NewType,
        _user: &Self::PermissionFilter,
    ) -> Result<Self::Type, Self::Error> {
        let ven: Ven = sqlx::query_as!(
            PostgresVen,
            r#"
            INSERT INTO ven (
                id,
                created_date_time,
                modification_date_time,
                ven_name,
                attributes,
                targets
            )
            VALUES (gen_random_uuid(), now(), now(), $1, $2, $3)
            RETURNING
                id,
                created_date_time,
                modification_date_time,
                ven_name,
                attributes,
                targets
            "#,
            new.ven_name,
            to_json_value(new.attributes)?,
            to_json_value(new.targets)?,
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?;

        Ok(ven)
    }

    async fn retrieve(
        &self,
        id: &Self::Id,
        _user: &Self::PermissionFilter,
    ) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresVen,
            r#"
            SELECT
                id,
                created_date_time,
                modification_date_time,
                ven_name,
                attributes,
                targets
            FROM ven
            WHERE id = $1
            "#,
            id.as_str(),
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
        let pg_filter: PostgresFilter = filter.into();
        trace!(?pg_filter);

        Ok(sqlx::query_as!(
            PostgresVen,
            r#"
            SELECT
                v.id AS "id!", 
                v.created_date_time AS "created_date_time!", 
                v.modification_date_time AS "modification_date_time!",
                v.ven_name AS "ven_name!",
                v.attributes,
                v.targets
            FROM ven v
            LEFT JOIN resource r ON r.ven_id = v.id
            WHERE ($1::text[] IS NULL OR v.ven_name = ANY($1))
              AND ($2::text[] IS NULL OR r.resource_name = ANY($2))
              AND ($3::jsonb = '[]'::jsonb OR $3::jsonb <@ v.targets)
            OFFSET $4 LIMIT $5
            "#,
            pg_filter.ven_names,
            pg_filter.resource_names,
            serde_json::to_value(pg_filter.targets)
                .map_err(AppError::SerdeJsonInternalServerError)?,
            pg_filter.skip,
            pg_filter.limit,
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
        let ven: Ven = sqlx::query_as!(
            PostgresVen,
            r#"
            UPDATE ven
            SET modification_date_time = now(),
                ven_name = $2,
                attributes = $3,
                targets = $4
            WHERE id = $1
            RETURNING
                id,
                created_date_time,
                modification_date_time,
                ven_name,
                attributes,
                targets
            "#,
            id.as_str(),
            new.ven_name,
            to_json_value(new.attributes)?,
            to_json_value(new.targets)?
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?;

        Ok(ven)
    }

    async fn delete(
        &self,
        id: &Self::Id,
        _user: &Self::PermissionFilter,
    ) -> Result<Self::Type, Self::Error> {
        Ok(sqlx::query_as!(
            PostgresVen,
            r#"
            DELETE FROM ven v
            WHERE v.id = $1
            RETURNING
                v.id,
                v.created_date_time,
                v.modification_date_time,
                v.ven_name,
                v.attributes,
                v.targets
            "#,
            id.as_str(),
        )
        .fetch_one(&self.db)
        .await?
        .try_into()?)
    }
}
