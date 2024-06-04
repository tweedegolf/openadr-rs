use std::collections::hash_map::Entry;

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use reqwest::StatusCode;

use openadr::wire::program::{ProgramContent, ProgramId, QueryParams};
use openadr::wire::Program;

use crate::api::{AppResponse, ValidatedQuery};
use crate::error::AppError;
use crate::error::AppError::NotFound;
use crate::state::AppState;

pub async fn get_all(
    State(state): State<AppState>,
    // TODO handle query params
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Program>> {
    Ok(Json(
        state.programs.read().await.values().cloned().collect(),
    ))
}

pub async fn get(State(state): State<AppState>, Path(id): Path<ProgramId>) -> AppResponse<Program> {
    Ok(Json(
        state
            .programs
            .read()
            .await
            .get(&id)
            .ok_or(NotFound)?
            .clone(),
    ))
}

// TODO
//   '409':
//   description: Conflict. Implementation dependent response if program with the same programName exists.
//   content:
//        application/json:
//        schema:
//        $ref: '#/components/schemas/problem'
pub async fn add(
    State(state): State<AppState>,
    Json(new_program): Json<ProgramContent>,
) -> Result<(StatusCode, Json<Program>), AppError> {
    let program = Program::new(new_program);
    state
        .programs
        .write()
        .await
        .insert(program.id.clone(), program.clone());
    Ok((StatusCode::CREATED, Json(program)))
}

// TODO
//   '409':
//   description: Conflict. Implementation dependent response if program with the same programName exists.
//   content:
//        application/json:
//        schema:
//        $ref: '#/components/schemas/problem'
pub async fn edit(
    State(state): State<AppState>,
    Path(id): Path<ProgramId>,
    Json(content): Json<ProgramContent>,
) -> AppResponse<Program> {
    let mut map = state.programs.write().await;
    match map.entry(id) {
        Entry::Occupied(mut entry) => {
            let p = entry.get_mut();
            p.content = content;
            p.modification_date_time = Utc::now();
            Ok(Json(p.clone()))
        }
        Entry::Vacant(_) => Err(NotFound),
    }
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<ProgramId>,
) -> AppResponse<Program> {
    match state.programs.write().await.remove(&id) {
        None => Err(NotFound),
        Some(removed) => Ok(Json(removed)),
    }
}
