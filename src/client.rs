use std::sync::Arc;

use url::Url;

use crate::wire::program::{ProgramContent, ProgramId};
use crate::wire::{target::TargetLabel, Program};
use crate::Result;

pub struct Client {
    client_ref: Arc<ClientRef>,
}

pub struct ClientRef {
    client: reqwest::Client,
    base_url: url::Url,
}

impl ClientRef {
    pub async fn get<'a, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: Option<impl serde::Serialize>,
    ) -> Result<T> {
        let url = self.base_url.join(path)?;
        let mut request = self.client.get(url);
        if let Some(query) = query {
            request = request.query(&query);
        }
        request = request.header("Accept", "application/json");
        let res = request.send().await?;

        // handle any errors returned by the server
        if !res.status().is_success() {
            let problem = res.json::<crate::wire::Problem>().await?;
            return Err(crate::Error::from(problem));
        }

        Ok(res.json().await?)
    }

    pub async fn post<'a, S: serde::ser::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &S,
    ) -> Result<T> {
        let url = self.base_url.join(path)?;
        let res = self.client.post(url).json(body).send().await?;

        // handle any errors returned by the server
        if !res.status().is_success() {
            let problem = res.json::<crate::wire::Problem>().await?;
            return Err(crate::Error::from(problem));
        }

        Ok(res.json().await?)
    }

    pub async fn put<'a, S: serde::ser::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &S,
    ) -> Result<T> {
        let url = self.base_url.join(path)?;
        let res = self.client.put(url).json(body).send().await?;

        // handle any errors returned by the server
        if !res.status().is_success() {
            let problem = res.json::<crate::wire::Problem>().await?;
            return Err(crate::Error::from(problem));
        }

        Ok(res.json().await?)
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = self.base_url.join(path)?;
        let res = self.client.delete(url).send().await?;

        // handle any errors returned by the server
        if !res.status().is_success() {
            let problem = res.json::<crate::wire::Problem>().await?;
            return Err(crate::Error::from(problem));
        }

        Ok(())
    }
}

impl Client {
    pub fn new(base_url: Url) -> Client {
        let client = reqwest::Client::new();
        Self::with_reqwest(base_url, client)
    }

    pub fn with_reqwest(base_url: Url, client: reqwest::Client) -> Client {
        Client {
            client_ref: Arc::new(ClientRef { client, base_url }),
        }
    }

    /// Create a new program on the VTN
    pub async fn create_program(&self, program_data: ProgramContent) -> Result<ProgramClient> {
        let program = self.client_ref.post("programs", &program_data).await?;
        Ok(ProgramClient::from_program(
            self.client_ref.clone(),
            program,
        ))
    }

    /// Get a list of programs from the VTN with the given query parameters
    pub async fn get_programs(
        &self,
        target_type: Option<TargetLabel>,
        targets: &[&str],
        skip: usize,
        limit: usize,
    ) -> Result<Vec<ProgramClient>> {
        // convert query params
        let target_type_str = target_type.map(|t| t.to_string());
        let skip_str = skip.to_string();
        let limit_str = limit.to_string();

        // insert into query params
        let mut query = vec![];
        if let Some(target_type_ref) = &target_type_str {
            for target in targets {
                query.push(("targetValues", *target));
            }
            query.push(("targetType", target_type_ref));
        }
        query.push(("skip", &skip_str));
        query.push(("limit", &limit_str));

        // send request and return response
        let programs: Vec<Program> = self.client_ref.get("programs", Some(&query)).await?;
        Ok(programs
            .into_iter()
            .map(|program| ProgramClient::from_program(self.client_ref.clone(), program))
            .collect())
    }

    /// Get all programs from the VTN, trying to paginate whenever possible
    pub async fn get_all_programs(&self) -> Result<Vec<ProgramClient>> {
        const PAGE_SIZE: usize = 50;
        let mut programs = vec![];
        let mut page = 0;
        loop {
            // TODO: this pagination should really depend on that the server indicated there are more results
            let received = self
                .get_programs(None, &[], page * PAGE_SIZE, PAGE_SIZE)
                .await?;
            let received_all = received.len() < PAGE_SIZE;
            for program in received {
                programs.push(program);
            }

            if received_all {
                break;
            } else {
                page += 1;
            }
        }

        Ok(programs)
    }

    /// Get a program by name
    pub async fn get_program_by_name(&self, name: &str) -> Result<ProgramClient> {
        let mut programs = self
            .get_programs(Some(TargetLabel::ProgramName), &[name], 0, 2)
            .await?;
        if programs.is_empty() {
            Err(crate::Error::ProgramNotFound)
        } else if programs.len() > 1 {
            Err(crate::Error::DuplicateProgram)
        } else {
            Ok(programs.remove(0))
        }
    }

    /// Get a program by id
    pub async fn get_program_by_id(&self, id: &ProgramId) -> Result<ProgramClient> {
        let program = self
            .client_ref
            .get(&format!("programs/{}", id.0), None::<()>)
            .await?;

        Ok(ProgramClient::from_program(
            self.client_ref.clone(),
            program,
        ))
    }
}

pub struct ProgramClient {
    client: Arc<ClientRef>,
    data: Program,
}

impl ProgramClient {
    fn from_program(client: Arc<ClientRef>, program: Program) -> ProgramClient {
        ProgramClient {
            client,
            data: program,
        }
    }

    pub fn id(&self) -> &ProgramId {
        &self.data.id
    }

    pub fn created_date_time(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.data.created_date_time
    }

    pub fn modification_date_time(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.data.modification_date_time
    }

    pub fn data(&self) -> &ProgramContent {
        &self.data.content
    }

    pub fn data_mut(&mut self) -> &mut ProgramContent {
        &mut self.data.content
    }

    /// Save any modifications of the program to the VTN
    pub async fn update(&mut self) -> Result<()> {
        let res = self
            .client
            .put(&format!("programs/{}", self.id()), &self.data.content)
            .await?;
        self.data = res;
        Ok(())
    }

    /// Delete the program from the VTN
    pub async fn delete(self) -> Result<()> {
        self.client.delete(&format!("programs/{}", self.id())).await
    }

    pub async fn get_events(
        &self,
        target_type: Option<TargetLabel>,
        targets: &[&str],
        skip: usize,
        limit: usize,
    ) -> Result<Vec<crate::wire::Event>> {
        // convert query params
        let target_type_str = target_type.map(|t| t.to_string());
        let skip_str = skip.to_string();
        let limit_str = limit.to_string();

        // insert into query params
        let mut query = vec![];
        if let Some(target_type_ref) = &target_type_str {
            for target in targets {
                query.push(("targetValues", *target));
            }
            query.push(("targetType", target_type_ref));
        }
        query.push(("skip", &skip_str));
        query.push(("limit", &limit_str));

        // send request and return response
        self.client.get("programs", Some(&query)).await
    }
}
