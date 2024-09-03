use crate::data_source::{AuthInfo, AuthSource};
use axum::async_trait;
use tokio::sync::RwLock;

#[async_trait]
impl AuthSource for RwLock<Vec<AuthInfo>> {
    async fn get_user(&self, client_id: &str, client_secret: &str) -> Option<AuthInfo> {
        self.read()
            .await
            .iter()
            .find(|auth| auth.client_id == client_id && auth.client_secret == client_secret)
            .cloned()
    }
}
