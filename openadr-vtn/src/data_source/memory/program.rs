use crate::api::program::QueryParams;
use crate::data_source::{Crud, ProgramCrud};
use crate::error::AppError;
use axum::async_trait;
use chrono::Utc;
use openadr_wire::program::{ProgramContent, ProgramId};
use openadr_wire::Program;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn};

impl ProgramCrud for RwLock<HashMap<ProgramId, Program>> {}

#[async_trait]
impl Crud for RwLock<HashMap<ProgramId, Program>> {
    type Type = Program;
    type Id = ProgramId;
    type NewType = ProgramContent;
    type Error = AppError;
    type Filter = QueryParams;

    async fn create(&self, new: Self::NewType) -> Result<Self::Type, Self::Error> {
        if let Some(conflict) = self
            .read()
            .await
            .values()
            .find(|p| p.content.program_name == new.program_name)
        {
            warn!(id=%conflict.id, program_name=%new.program_name, "Conflicting program_name");
            return Err(AppError::Conflict(format!(
                "Program with id {} has the same name",
                conflict.id
            )));
        }

        let program = Program::new(new);
        self.write()
            .await
            .insert(program.id.clone(), program.clone());

        info!(%program.id,
            program.program_name=program.content.program_name,
            "program created"
        );

        Ok(program)
    }

    async fn retrieve(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        self.read().await.get(id).cloned().ok_or(AppError::NotFound)
    }

    async fn retrieve_all(&self, filter: &Self::Filter) -> Result<Vec<Self::Type>, Self::Error> {
        self.read()
            .await
            .values()
            .filter_map(|program| match filter.matches(program) {
                Ok(true) => Some(Ok(program.clone())),
                Ok(false) => None,
                Err(err) => Some(Err(err)),
            })
            .skip(filter.skip as usize)
            .take(filter.limit as usize)
            .collect::<Result<Vec<_>, AppError>>()
    }

    async fn update(
        &self,
        id: &Self::Id,
        content: Self::NewType,
    ) -> Result<Self::Type, Self::Error> {
        if let Some((_, conflict)) =
            self.read().await.iter().find(|(inner_id, p)| {
                id != *inner_id && p.content.program_name == content.program_name
            })
        {
            warn!(updated=%id, conflicting=%conflict.id, program_name=%content.program_name, "Conflicting program_name");
            return Err(AppError::Conflict(format!(
                "Program with id {} has the same name",
                conflict.id
            )));
        }

        match self.write().await.get_mut(id) {
            Some(occupied) => {
                occupied.content = content;
                occupied.modification_date_time = Utc::now();
                Ok(occupied.clone())
            }
            None => Err(AppError::NotFound),
        }
    }

    async fn delete(&self, id: &Self::Id) -> Result<Self::Type, Self::Error> {
        match self.write().await.remove(id) {
            Some(program) => Ok(program),
            None => Err(AppError::NotFound),
        }
    }
}
