use std::sync::Arc;

use url::Url;

pub use crate::wire::{
    event::EventContent,
    program::{ProgramContent, ProgramId},
};
use crate::wire::{
    event::EventObjectType,
    report::{ReportContent, ReportObjectType},
    Event, Report,
};
use crate::wire::{target::TargetLabel, Program};
use crate::Result;

/// Target for a query to the VTN
#[derive(Copy, Clone, Debug)]
pub enum Target<'a> {
    /// Target by a specific program name
    Program(&'a str),

    /// Target by a list of program names
    Programs(&'a [&'a str]),

    /// Target by a specific event name
    Event(&'a str),

    /// Target by a list of event names
    Events(&'a [&'a str]),

    /// Target by a specific VEN name
    VEN(&'a str),

    /// Target by a list of VEN names
    VENs(&'a [&'a str]),

    /// Target by a specific group name
    Group(&'a str),

    /// Target by a list of group names
    Groups(&'a [&'a str]),

    /// Target by a specific resource name
    Resource(&'a str),

    /// Target by a list of resource names
    Resources(&'a [&'a str]),

    /// Target by a specific service area
    ServiceArea(&'a str),

    /// Target by a list of service areas
    ServiceAreas(&'a [&'a str]),

    /// Target by a specific power service location
    PowerServiceLocation(&'a str),

    /// Target by a list of power service locations
    PowerServiceLocations(&'a [&'a str]),

    /// Target using some other kind of privately defined target type, using a single target value
    Other(&'a str, &'a str),

    /// Target using some other kind of privately defined target type, with a list of values
    Others(&'a str, &'a [&'a str]),
}

impl<'a> Target<'a> {
    /// Get the target label for this specific target
    pub fn target_label(&self) -> TargetLabel {
        match self {
            Target::Program(_) | Target::Programs(_) => TargetLabel::ProgramName,
            Target::Event(_) | Target::Events(_) => TargetLabel::EventName,
            Target::VEN(_) | Target::VENs(_) => TargetLabel::VENName,
            Target::Group(_) | Target::Groups(_) => TargetLabel::Group,
            Target::Resource(_) | Target::Resources(_) => TargetLabel::ResourceName,
            Target::ServiceArea(_) | Target::ServiceAreas(_) => TargetLabel::ServiceArea,
            Target::PowerServiceLocation(_) | Target::PowerServiceLocations(_) => {
                TargetLabel::PowerServiceLocation
            }
            Target::Other(p, _) | Target::Others(p, _) => TargetLabel::Private(p.to_string()),
        }
    }

    /// Get the list of target values for this specific target
    pub fn target_values(&self) -> &[&str] {
        match self {
            Target::Program(v) => std::slice::from_ref(v),
            Target::Programs(v) => v,
            Target::Event(v) => std::slice::from_ref(v),
            Target::Events(v) => v,
            Target::VEN(v) => std::slice::from_ref(v),
            Target::VENs(v) => v,
            Target::Group(v) => std::slice::from_ref(v),
            Target::Groups(v) => v,
            Target::Resource(v) => std::slice::from_ref(v),
            Target::Resources(v) => v,
            Target::ServiceArea(v) => std::slice::from_ref(v),
            Target::ServiceAreas(v) => v,
            Target::PowerServiceLocation(v) => std::slice::from_ref(v),
            Target::PowerServiceLocations(v) => v,
            Target::Other(_, v) => std::slice::from_ref(v),
            Target::Others(_, v) => v,
        }
    }
}

/// Client used for interaction with a VTN.
///
/// Can be used to implement both, the VEN and the business logic
pub struct Client {
    client_ref: Arc<ClientRef>,
}

struct ClientRef {
    client: reqwest::Client,
    base_url: url::Url,
    default_page_size: usize,
}

impl std::fmt::Debug for ClientRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ClientRef")
            .field(&self.base_url.to_string())
            .finish()
    }
}

impl ClientRef {
    async fn request<T: serde::de::DeserializeOwned>(
        mut request: reqwest::RequestBuilder,
        query: &[(&str, &str)],
    ) -> Result<T> {
        request = request.header("Accept", "application/json");
        if !query.is_empty() {
            request = request.query(&query);
        }
        let res = request.send().await?;

        // handle any errors returned by the server
        if !res.status().is_success() {
            let problem = res.json::<crate::wire::Problem>().await?;
            return Err(crate::Error::from(problem));
        }

        Ok(res.json().await?)
    }

    pub async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T> {
        let url = self.base_url.join(path)?;
        let request = self.client.get(url);
        ClientRef::request(request, query).await
    }

    pub async fn post<S: serde::ser::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &S,
        query: &[(&str, &str)],
    ) -> Result<T> {
        let url = self.base_url.join(path)?;
        let request = self.client.post(url).json(body);
        ClientRef::request(request, query).await
    }

    pub async fn put<S: serde::ser::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &S,
        query: &[(&str, &str)],
    ) -> Result<T> {
        let url = self.base_url.join(path)?;
        let request = self.client.put(url).json(body);
        ClientRef::request(request, query).await
    }

    pub async fn delete(&self, path: &str, query: &[(&str, &str)]) -> Result<()> {
        let url = self.base_url.join(path)?;
        let request = self.client.delete(url);
        ClientRef::request(request, query).await
    }
}

impl Client {
    /// Create a new client for a VTN located at the specified URL
    pub fn new(base_url: Url) -> Client {
        let client = reqwest::Client::new();
        Self::with_reqwest(base_url, client)
    }

    /// Create a new client, but use the specific reqwest client instead of
    /// the default one. This allows you to configure proxy settings, timeouts, etc.
    pub fn with_reqwest(base_url: Url, client: reqwest::Client) -> Client {
        Client {
            client_ref: Arc::new(ClientRef {
                client,
                base_url,
                default_page_size: 50,
            }),
        }
    }

    /// Create a new program on the VTN
    pub async fn create_program(&self, program_data: ProgramContent) -> Result<ProgramClient> {
        let program = self.client_ref.post("programs", &program_data, &[]).await?;
        Ok(ProgramClient::from_program(
            self.client_ref.clone(),
            program,
        ))
    }

    /// Get a list of programs from the VTN with the given query parameters
    async fn get_programs_req(
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
        let programs: Vec<Program> = self.client_ref.get("programs", &query).await?;
        Ok(programs
            .into_iter()
            .map(|program| ProgramClient::from_program(self.client_ref.clone(), program))
            .collect())
    }

    /// Get a single program from the VTN that matches the given target
    pub async fn get_program(&self, target: Target<'_>) -> Result<ProgramClient> {
        let mut programs = self
            .get_programs_req(Some(target.target_label()), target.target_values(), 0, 2)
            .await?;
        if programs.is_empty() {
            Err(crate::Error::ObjectNotFound)
        } else if programs.len() > 1 {
            Err(crate::Error::DuplicateObject)
        } else {
            Ok(programs.remove(0))
        }
    }

    /// Get a list of programs from the VTN with the given query parameters
    pub async fn get_program_list(&self, target: Target<'_>) -> Result<Vec<ProgramClient>> {
        let page_size = self.client_ref.default_page_size;
        let mut programs = vec![];
        let mut page = 0;
        loop {
            let received = self
                .get_programs_req(
                    Some(target.target_label()),
                    target.target_values(),
                    page * page_size,
                    page_size,
                )
                .await?;
            let received_all = received.len() < page_size;
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

    /// Get all programs from the VTN, trying to paginate whenever possible
    pub async fn get_all_programs(&self) -> Result<Vec<ProgramClient>> {
        let page_size = self.client_ref.default_page_size;
        let mut programs = vec![];
        let mut page = 0;
        loop {
            // TODO: this pagination should really depend on that the server indicated there are more results
            let received = self
                .get_programs_req(None, &[], page * page_size, page_size)
                .await?;
            let received_all = received.len() < page_size;
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
        self.get_program(Target::Program(name)).await
    }

    /// Get a program by id
    pub async fn get_program_by_id(&self, id: &ProgramId) -> Result<ProgramClient> {
        let program = self
            .client_ref
            .get(&format!("programs/{}", id.0), &[])
            .await?;

        Ok(ProgramClient::from_program(
            self.client_ref.clone(),
            program,
        ))
    }
}

/// A client for interacting with the data in a specific program and the events
/// contained in the program.
#[derive(Debug)]
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

    /// Get the id of the program
    pub fn id(&self) -> &ProgramId {
        &self.data.id
    }

    /// Get the time the program was created on the VTN
    pub fn created_date_time(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.data.created_date_time
    }

    /// Get the time the program was last modified on the VTN
    pub fn modification_date_time(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.data.modification_date_time
    }

    /// Read the data of the program
    pub fn data(&self) -> &ProgramContent {
        &self.data.content
    }

    /// Modify the data of the program, make sure to update the program on the
    /// VTN once your modifications are complete.
    pub fn data_mut(&mut self) -> &mut ProgramContent {
        &mut self.data.content
    }

    /// Save any modifications of the program to the VTN
    pub async fn update(&mut self) -> Result<()> {
        let res = self
            .client
            .put(&format!("programs/{}", self.id()), &self.data.content, &[])
            .await?;
        self.data = res;
        Ok(())
    }

    /// Delete the program from the VTN
    pub async fn delete(self) -> Result<()> {
        self.client
            .delete(&format!("programs/{}", self.id()), &[])
            .await
    }

    /// Create a new event on the VTN
    pub async fn create_event(&self, event_data: EventContent) -> Result<EventClient> {
        if &event_data.program_id != self.id() {
            return Err(crate::Error::InvalidParentObject);
        }
        let event = self.client.post("events", &event_data, &[]).await?;
        Ok(EventClient::from_event(self.client.clone(), event))
    }

    /// Create a new event object within the program
    pub fn new_event(&self) -> EventContent {
        EventContent {
            object_type: Some(EventObjectType::Event),
            program_id: self.id().clone(),
            event_name: None,
            priority: None,
            targets: None,
            report_descriptors: None,
            payload_descriptors: None,
            interval_period: None,
            intervals: vec![],
        }
    }

    async fn get_events_req(
        &self,
        target_type: Option<TargetLabel>,
        targets: &[&str],
        skip: usize,
        limit: usize,
    ) -> Result<Vec<EventClient>> {
        // convert query params
        let target_type_str = target_type.map(|t| t.to_string());
        let skip_str = skip.to_string();
        let limit_str = limit.to_string();

        // insert into query params
        let mut query = vec![("programID", self.id().as_str())];
        if let Some(target_type_ref) = &target_type_str {
            for target in targets {
                query.push(("targetValues", *target));
            }
            query.push(("targetType", target_type_ref));
        }
        query.push(("skip", &skip_str));
        query.push(("limit", &limit_str));

        // send request and return response
        let events: Vec<Event> = self.client.get("events", &query).await?;
        Ok(events
            .into_iter()
            .map(|event| EventClient::from_event(self.client.clone(), event))
            .collect())
    }

    /// Get a single event from the VTN that matches the given target
    pub async fn get_event(&self, target: Target<'_>) -> Result<EventClient> {
        let mut events = self
            .get_events_req(Some(target.target_label()), target.target_values(), 0, 2)
            .await?;
        if events.is_empty() {
            Err(crate::Error::ObjectNotFound)
        } else if events.len() > 1 {
            Err(crate::Error::DuplicateObject)
        } else {
            Ok(events.remove(0))
        }
    }

    /// Get a list of events from the VTN with the given query parameters
    pub async fn get_event_list(&self, target: Target<'_>) -> Result<Vec<EventClient>> {
        let page_size = self.client.default_page_size;
        let mut events = vec![];
        let mut page = 0;
        loop {
            let received = self
                .get_events_req(
                    Some(target.target_label()),
                    target.target_values(),
                    page * page_size,
                    page_size,
                )
                .await?;
            let received_all = received.len() < page_size;
            for event in received {
                events.push(event);
            }

            if received_all {
                break;
            } else {
                page += 1;
            }
        }

        Ok(events)
    }

    /// Get all events from the VTN, trying to paginate whenever possible
    pub async fn get_all_events(&self) -> Result<Vec<EventClient>> {
        let page_size = self.client.default_page_size;
        let mut events = vec![];
        let mut page = 0;
        loop {
            // TODO: this pagination should really depend on that the server indicated there are more results
            let received = self
                .get_events_req(None, &[], page * page_size, page_size)
                .await?;
            let received_all = received.len() < page_size;
            for event in received {
                events.push(event);
            }

            if received_all {
                break;
            } else {
                page += 1;
            }
        }

        Ok(events)
    }
}

#[derive(Debug)]
pub struct EventClient {
    client: Arc<ClientRef>,
    data: Event,
}

impl EventClient {
    fn from_event(client: Arc<ClientRef>, event: Event) -> EventClient {
        EventClient {
            client,
            data: event,
        }
    }

    pub fn id(&self) -> &crate::wire::event::EventId {
        &self.data.id
    }

    pub fn created_date_time(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.data.created_date_time
    }

    pub fn modification_date_time(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.data.modification_date_time
    }

    pub fn data(&self) -> &EventContent {
        &self.data.content
    }

    pub fn data_mut(&mut self) -> &mut EventContent {
        &mut self.data.content
    }

    /// Save any modifications of the event to the VTN
    pub async fn update(&mut self) -> Result<()> {
        let res = self
            .client
            .put(&format!("events/{}", self.id()), &self.data.content, &[])
            .await?;
        self.data = res;
        Ok(())
    }

    /// Delete the event from the VTN
    pub async fn delete(self) -> Result<()> {
        self.client
            .delete(&format!("events/{}", self.id()), &[])
            .await
    }

    /// Create a new report object
    pub fn new_report(&self) -> ReportContent {
        ReportContent {
            object_type: Some(ReportObjectType::Report),
            program_id: self.data().program_id.clone(),
            event_id: self.id().clone(),
            client_name: "".to_string(),
            report_name: None,
            payload_descriptors: None,
            resources: vec![],
        }
    }

    /// Create a new report for the event
    pub async fn create_report(&self, report_data: ReportContent) -> Result<ReportClient> {
        if report_data.program_id != self.data().program_id {
            return Err(crate::Error::InvalidParentObject);
        }

        if &report_data.event_id != self.id() {
            return Err(crate::Error::InvalidParentObject);
        }

        let report = self.client.post("events", &report_data, &[]).await?;
        Ok(ReportClient::from_report(self.client.clone(), report))
    }

    async fn get_reports_req(
        &self,
        client_name: Option<&str>,
        skip: usize,
        limit: usize,
    ) -> Result<Vec<ReportClient>> {
        let skip_str = skip.to_string();
        let limit_str = limit.to_string();

        let mut query = vec![
            ("programID", self.data().program_id.as_str()),
            ("eventID", self.id().as_str()),
            ("skip", &skip_str),
            ("limit", &limit_str),
        ];

        if let Some(client_name) = client_name {
            query.push(("clientName", client_name));
        }

        let reports: Vec<Report> = self.client.get("reports", &query).await?;
        Ok(reports
            .into_iter()
            .map(|report| ReportClient::from_report(self.client.clone(), report))
            .collect())
    }

    /// Get all reports from the VTN for a specific client, trying to paginate whenever possible
    pub async fn get_client_reports(&self, client_name: &str) -> Result<Vec<ReportClient>> {
        let page_size = self.client.default_page_size;
        let mut reports = vec![];
        let mut page = 0;
        loop {
            let received = self
                .get_reports_req(Some(client_name), page * page_size, page_size)
                .await?;
            let received_all = received.len() < page_size;
            for report in received {
                reports.push(report);
            }

            if received_all {
                break;
            } else {
                page += 1;
            }
        }

        Ok(reports)
    }

    /// Get all reports from the VTN, trying to paginate whenever possible
    pub async fn get_all_reports(&self) -> Result<Vec<ReportClient>> {
        let page_size = self.client.default_page_size;
        let mut reports = vec![];
        let mut page = 0;
        loop {
            let received = self
                .get_reports_req(None, page * page_size, page_size)
                .await?;
            let received_all = received.len() < page_size;
            for report in received {
                reports.push(report);
            }

            if received_all {
                break;
            } else {
                page += 1;
            }
        }

        Ok(reports)
    }
}

#[derive(Debug)]
pub struct ReportClient {
    client: Arc<ClientRef>,
    data: Report,
}

impl ReportClient {
    fn from_report(client: Arc<ClientRef>, report: Report) -> ReportClient {
        ReportClient {
            client,
            data: report,
        }
    }

    pub fn id(&self) -> &crate::wire::report::ReportId {
        &self.data.id
    }

    pub fn created_date_time(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.data.created_date_time
    }

    pub fn modification_date_time(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.data.modification_date_time
    }

    pub fn data(&self) -> &ReportContent {
        &self.data.content
    }

    pub fn data_mut(&mut self) -> &mut ReportContent {
        &mut self.data.content
    }

    /// Save any modifications of the report to the VTN
    pub async fn update(&mut self) -> Result<()> {
        let res = self
            .client
            .put(&format!("reports/{}", self.id()), &self.data.content, &[])
            .await?;
        self.data = res;
        Ok(())
    }

    /// Delete the report from the VTN
    pub async fn delete(self) -> Result<()> {
        self.client
            .delete(&format!("reports/{}", self.id()), &[])
            .await
    }
}
