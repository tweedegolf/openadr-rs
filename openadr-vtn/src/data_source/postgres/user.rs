use crate::{
    data_source::{postgres::PgId, AuthInfo, AuthSource},
    jwt::AuthRole,
};
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

#[derive(Debug)]
struct MaybePgId {
    id: Option<String>,
}

#[async_trait]
impl AuthSource for PgAuthSource {
    async fn get_user(&self, client_id: &str, client_secret: &str) -> Option<AuthInfo> {
        let vens = sqlx::query_as!(
            PgId,
            r#"
            SELECT ven_id AS id
            FROM "user" u
              JOIN user_credentials c ON c.user_id = u.id
              JOIN user_ven v ON v.user_id = u.id 
            WHERE client_id = $1
              AND client_secret = $2
            "#,
            client_id,
            client_secret
        )
        .fetch_all(&self.db)
        .await
        .ok();

        let businesses = sqlx::query_as!(
            MaybePgId,
            r#"
            SELECT ub.business_id AS id 
            FROM user_business ub
                JOIN "user" u ON u.id = ub.user_id
                JOIN user_credentials c ON c.user_id = u.id
            WHERE client_id = $1
              AND client_secret = $2
            "#,
            client_id,
            client_secret
        )
        .fetch_all(&self.db)
        .await
        .ok();

        let mut ven_roles = vens
            .map(|vens| {
                vens.into_iter()
                    .map(|ven| AuthRole::VEN(ven.id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut business_roles = businesses
            .map(|vens| {
                vens.into_iter()
                    .map(|ven| {
                        if let Some(id) = ven.id {
                            AuthRole::Business(id)
                        } else {
                            AuthRole::AnyBusiness
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        ven_roles.append(&mut business_roles);

        if ven_roles.is_empty() {
            None
        } else {
            Some(AuthInfo {
                client_id: client_id.to_string(),
                roles: ven_roles,
            })
        }
    }
}
