use crate::{
    api::AppResponse,
    data_source::{AuthSource, UserDetails},
    jwt::{AuthRole, UserManagerUser},
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde_with::serde_derive::Deserialize;
use std::sync::Arc;
use tracing::{info, trace};

#[derive(Deserialize)]
pub struct NewUser {
    reference: String,
    description: Option<String>,
    roles: Vec<AuthRole>,
}

#[derive(Deserialize)]
pub struct NewCredential {
    client_id: String,
    client_secret: String,
}

pub async fn get_all(
    State(auth_source): State<Arc<dyn AuthSource>>,
    UserManagerUser(_): UserManagerUser,
) -> AppResponse<Vec<UserDetails>> {
    let users = auth_source.get_all_users().await?;

    trace!("received {} users", users.len());
    Ok(Json(users))
}

pub async fn get(
    State(auth_source): State<Arc<dyn AuthSource>>,
    Path(id): Path<String>,
    UserManagerUser(_): UserManagerUser,
) -> AppResponse<UserDetails> {
    let user = auth_source.get_user(&id).await?;
    trace!(user_id = user.id(), "received user");
    Ok(Json(user))
}

pub async fn add_user(
    State(auth_source): State<Arc<dyn AuthSource>>,
    UserManagerUser(_): UserManagerUser,
    Json(new_user): Json<NewUser>,
) -> AppResponse<UserDetails> {
    let user = auth_source
        .add_user(
            &new_user.reference,
            new_user.description.as_deref(),
            &new_user.roles,
        )
        .await?;
    info!(user_id = user.id(), "created new user");
    Ok(Json(user))
}

pub async fn add_credential(
    State(auth_source): State<Arc<dyn AuthSource>>,
    Path(id): Path<String>,
    UserManagerUser(_): UserManagerUser,
    Json(new): Json<NewCredential>,
) -> AppResponse<UserDetails> {
    let user = auth_source
        .add_credentials(&id, &new.client_id, &new.client_secret)
        .await?;
    info!(
        user_id = id,
        client_id = new.client_id,
        "created new credential for user"
    );
    Ok(Json(user))
}

pub async fn edit(
    State(auth_source): State<Arc<dyn AuthSource>>,
    Path(id): Path<String>,
    UserManagerUser(_): UserManagerUser,
    Json(modified): Json<NewUser>,
) -> AppResponse<UserDetails> {
    let user = auth_source
        .edit_user(
            &id,
            &modified.reference,
            modified.description.as_deref(),
            &modified.roles,
        )
        .await?;

    info!(user_id = user.id(), "updated user");
    Ok(Json(user))
}

pub async fn delete_user(
    State(auth_source): State<Arc<dyn AuthSource>>,
    Path(id): Path<String>,
    UserManagerUser(_): UserManagerUser,
) -> AppResponse<UserDetails> {
    let user = auth_source.remove_user(&id).await?;
    info!(user_id = user.id(), "deleted user");
    Ok(Json(user))
}

pub async fn delete_credential(
    State(auth_source): State<Arc<dyn AuthSource>>,
    Path((user_id, client_id)): Path<(String, String)>,
    UserManagerUser(_): UserManagerUser,
) -> AppResponse<UserDetails> {
    let user = auth_source.remove_credentials(&user_id, &client_id).await?;
    info!(user_id = user.id(), client_id, "deleted credential");
    Ok(Json(user))
}
