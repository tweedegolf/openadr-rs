use std::collections::hash_map::Entry;

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use reqwest::StatusCode;
use serde::Deserialize;
use validator::Validate;

use openadr::wire::program::{ProgramContent, ProgramId};
use openadr::wire::target::TargetLabel;
use openadr::wire::{Pagination, Program};

use crate::api::{AppResponse, ValidatedQuery};
use crate::error::AppError;
use crate::error::AppError::NotFound;
use crate::state::AppState;

pub async fn get_all(
    State(state): State<AppState>,
    ValidatedQuery(query_params): ValidatedQuery<QueryParams>,
) -> AppResponse<Vec<Program>> {
    let programs = state
        .programs
        .read()
        .await
        .values()
        .filter_map(|program| match query_params.matches(program) {
            Ok(true) => Some(Ok(program.clone())),
            Ok(false) => None,
            Err(err) => Some(Err(err)),
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    let pagination = query_params.pagination;

    Ok(Json(
        programs
            .get(pagination.skip as usize..(pagination.skip + pagination.limit) as usize)
            .unwrap_or(&[])
            .to_vec(),
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

#[derive(Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct QueryParams {
    target_type: Option<TargetLabel>,
    target_values: Option<Vec<String>>,
    #[serde(flatten)]
    #[validate(nested)]
    pagination: Pagination,
}

impl QueryParams {
    pub fn matches(&self, program: &Program) -> Result<bool, AppError> {
        if let Some(target_type) = self.target_type.clone() {
            return match target_type {
                TargetLabel::ProgramName => Ok(self
                    .target_values
                    .clone()
                    .ok_or(AppError::BadRequest(
                        "If targetType is specified, targetValues must be specified as well",
                    ))?
                    .into_iter()
                    .any(|name| name == program.content.program_name)),
                _ => Err(AppError::NotImplemented(
                    "Program can only be filtered by name",
                )),
            };
        }
        Ok(true)
    }
}
