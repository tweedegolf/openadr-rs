use crate::data_source::{AuthInfo, AuthSource};
use axum::async_trait;
use sqlx::PgPool;

pub struct PgAuthSource {
    db: PgPool,
}

impl From<PgPool> for PgAuthSource {
    fn from(db: PgPool) -> Self {
        Self { db }
    }
}

struct PgUser {
    client_id: String,
    client_secret: String,
    roles: Vec<serde_json::Value>,
}

impl TryFrom<PgUser> for AuthInfo {
    type Error = serde_json::Error;

    fn try_from(value: PgUser) -> Result<Self, Self::Error> {
        Ok(Self {
            client_id: value.client_id,
            client_secret: value.client_secret,
            roles: value
                .roles
                .into_iter()
                .map(serde_json::from_value)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

#[async_trait]
impl AuthSource for PgAuthSource {
    async fn get_user(&self, client_id: &str, client_secret: &str) -> Option<AuthInfo> {
        sqlx::query_as!(
            PgUser,
            r#"
            SELECT client_id,
                   client_secret,
                   array_agg(role) AS "roles!"
            FROM "user" u
              INNER JOIN user_credentials c ON c.user_id = u.id
              INNER JOIN user_roles r ON r.user_id = u.id 
            WHERE client_id = $1
              AND client_secret = $2
            GROUP BY u.id, c.client_id
            "#,
            client_id,
            client_secret
        )
        .fetch_one(&self.db)
        .await
        .map(TryInto::<AuthInfo>::try_into)
        .map(Result::ok)
        .ok()
        .flatten()
    }
}
