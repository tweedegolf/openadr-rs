use std::sync::Arc;

use crate::{
    wire::{
        event::{EventObjectType, Priority},
        target::TargetLabel,
        Event, Program,
    },
    Error, EventClient, EventContent, ProgramContent, ProgramId, Result, Target, Timeline,
};

use super::ClientRef;

/// A client for interacting with the data in a specific program and the events
/// contained in the program.
#[derive(Debug)]
pub struct ProgramClient {
    client: Arc<ClientRef>,
    data: Program,
}

impl ProgramClient {
    pub(super) fn from_program(client: Arc<ClientRef>, program: Program) -> ProgramClient {
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
            priority: Priority::UNSPECIFIED,
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

    pub async fn get_timeline(&mut self) -> Result<Timeline> {
        let events = self.get_all_events().await?;
        let events = events.iter().map(|e| e.data()).collect();
        Timeline::from_events(self.data(), events).map_err(|_| Error::InvalidInterval)
    }
}
