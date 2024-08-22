use axum::http::StatusCode;

use openadr_client::{Error, PaginationOptions};
use openadr_wire::{
    event::{EventContent, Priority},
    program::ProgramId,
    target::TargetLabel,
};

mod common;

fn default_content() -> EventContent {
    EventContent {
        object_type: None,
        program_id: ProgramId::new("program_id").unwrap(),
        event_name: Some("event_name".to_string()),
        priority: Priority::MAX,
        report_descriptors: None,
        interval_period: None,
        intervals: vec![],
        payload_descriptors: None,
        targets: None,
    }
}

#[tokio::test]
async fn get() {
    let client = common::setup_client();
    let event_client = client.create_event(default_content()).await.unwrap();

    assert_eq!(event_client.content(), &default_content());
}

#[tokio::test]
async fn delete() {
    let client = common::setup_client();

    let event1 = EventContent {
        event_name: Some("event1".to_string()),
        ..default_content()
    };
    let event2 = EventContent {
        event_name: Some("event2".to_string()),
        ..default_content()
    };
    let event3 = EventContent {
        event_name: Some("event3".to_string()),
        ..default_content()
    };

    for content in [event1, event2.clone(), event3] {
        client.create_event(content).await.unwrap();
    }

    let pagination = PaginationOptions { skip: 0, limit: 2 };
    let mut events = client
        .get_events_request(Some(TargetLabel::EventName), &["event2"], pagination)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    let event = events.pop().unwrap();
    assert_eq!(event.content(), &event2);

    let removed = event.delete().await.unwrap();
    assert_eq!(removed.content, event2);

    let events = client.get_all_events().await.unwrap();
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn update() {
    let client = common::setup_client();

    let event1 = EventContent {
        event_name: Some("event1".to_string()),
        ..default_content()
    };

    let mut event = client.create_event(event1).await.unwrap();
    let creation_date_time = event.modification_date_time();

    let event2 = EventContent {
        event_name: Some("event1".to_string()),
        priority: Priority::MIN,
        ..default_content()
    };

    *event.content_mut() = event2.clone();
    event.update().await.unwrap();

    assert_eq!(event.content(), &event2);
    assert!(event.modification_date_time() > creation_date_time);
}

#[tokio::test]
async fn update_same_name() {
    let client = common::setup_client();

    let event1 = EventContent {
        event_name: Some("event1".to_string()),
        ..default_content()
    };

    let event2 = EventContent {
        event_name: Some("event2".to_string()),
        ..default_content()
    };

    let _event1 = client.create_event(event1).await.unwrap();
    let mut event2 = client.create_event(event2).await.unwrap();
    let creation_date_time = event2.modification_date_time();

    let content = EventContent {
        event_name: Some("event1".to_string()),
        priority: Priority::MIN,
        ..default_content()
    };

    // duplicate event names are fine
    *event2.content_mut() = content;
    event2.update().await.unwrap();

    assert!(event2.modification_date_time() > creation_date_time);
}

#[tokio::test]
async fn create_same_name() {
    let client = common::setup_client();

    let event1 = EventContent {
        event_name: Some("event1".to_string()),
        ..default_content()
    };

    // duplicate event names are fine
    let _ = client.create_event(event1.clone()).await.unwrap();
    let _ = client.create_event(event1).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn retrieve_all_with_filter() {
    let client = common::setup_client();

    let event1 = EventContent {
        program_id: ProgramId::new("program1").unwrap(),
        event_name: Some("event1".to_string()),
        ..default_content()
    };
    let event2 = EventContent {
        program_id: ProgramId::new("program2").unwrap(),
        event_name: Some("event2".to_string()),
        ..default_content()
    };
    let event3 = EventContent {
        program_id: ProgramId::new("program3").unwrap(),
        event_name: Some("event3".to_string()),
        ..default_content()
    };

    for content in [event1, event2, event3] {
        let _ = client.create_event(content).await.unwrap();
    }

    let events = client
        .get_events_request(None, &[], PaginationOptions { skip: 0, limit: 50 })
        .await
        .unwrap();
    assert_eq!(events.len(), 3);

    // skip
    let events = client
        .get_events_request(None, &[], PaginationOptions { skip: 1, limit: 50 })
        .await
        .unwrap();
    assert_eq!(events.len(), 2);

    // limit
    let events = client
        .get_events_request(None, &[], PaginationOptions { skip: 0, limit: 2 })
        .await
        .unwrap();
    assert_eq!(events.len(), 2);

    // event name
    let err = client
        .get_events_request(
            Some(TargetLabel::Private("NONSENSE".to_string())),
            &[],
            PaginationOptions { skip: 0, limit: 2 },
        )
        .await
        .unwrap_err();
    let Error::Problem(problem) = err else {
        unreachable!()
    };
    assert_eq!(problem.status, StatusCode::NOT_IMPLEMENTED);

    let events = client
        .get_events_request(
            Some(TargetLabel::ProgramName),
            &["program1", "program2"],
            PaginationOptions { skip: 0, limit: 50 },
        )
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
}
