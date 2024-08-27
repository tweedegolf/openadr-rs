use axum::http::StatusCode;

use openadr_client::{Error, Filter, PaginationOptions};
use openadr_wire::{program::ProgramContent, target::TargetLabel};

mod common;

fn default_content() -> ProgramContent {
    ProgramContent {
        object_type: None,
        program_name: "program_name".to_string(),
        program_long_name: Some("program_long_name".to_string()),
        retailer_name: Some("retailer_name".to_string()),
        retailer_long_name: Some("retailer_long_name".to_string()),
        program_type: None,
        country: None,
        principal_subdivision: None,
        time_zone_offset: None,
        interval_period: None,
        program_descriptions: None,
        binding_events: None,
        local_price: None,
        payload_descriptors: None,
        targets: None,
    }
}

#[tokio::test]
async fn get() {
    let client = common::setup_client();
    let program_client = client.create_program(default_content()).await.unwrap();

    assert_eq!(program_client.content(), &default_content());
}

#[tokio::test]
async fn delete() {
    let client = common::setup_client();

    let program1 = ProgramContent {
        program_name: "program1".to_string(),
        ..default_content()
    };
    let program2 = ProgramContent {
        program_name: "program2".to_string(),
        ..default_content()
    };
    let program3 = ProgramContent {
        program_name: "program3".to_string(),
        ..default_content()
    };

    for content in [program1, program2.clone(), program3] {
        client.create_program(content).await.unwrap();
    }

    let program = client.get_program_by_name("program2").await.unwrap();
    assert_eq!(program.content(), &program2);

    let removed = program.delete().await.unwrap();
    assert_eq!(removed.content, program2);

    let programs = client.get_all_programs().await.unwrap();
    assert_eq!(programs.len(), 2);
}

#[tokio::test]
async fn update() {
    let client = common::setup_client();

    let program1 = ProgramContent {
        program_name: "program1".to_string(),
        ..default_content()
    };

    let mut program = client.create_program(program1).await.unwrap();
    let creation_date_time = program.modification_date_time();

    let program2 = ProgramContent {
        program_name: "program1".to_string(),
        country: Some("NO".to_string()),
        ..default_content()
    };

    *program.content_mut() = program2.clone();
    program.update().await.unwrap();

    assert_eq!(program.content(), &program2);
    assert!(program.modification_date_time() > creation_date_time);
}

#[tokio::test]
async fn update_same_name() {
    let client = common::setup_client();

    let program1 = ProgramContent {
        program_name: "program1".to_string(),
        ..default_content()
    };

    let program2 = ProgramContent {
        program_name: "program2".to_string(),
        ..default_content()
    };

    let _program1 = client.create_program(program1).await.unwrap();
    let mut program2 = client.create_program(program2).await.unwrap();
    let creation_date_time = program2.modification_date_time();

    let content = ProgramContent {
        program_name: "program1".to_string(),
        country: Some("NO".to_string()),
        ..default_content()
    };

    *program2.content_mut() = content;

    let Error::Problem(problem) = program2.update().await.unwrap_err() else {
        unreachable!()
    };

    assert_eq!(problem.status, StatusCode::CONFLICT);
    assert!(program2.modification_date_time() == creation_date_time);
}

#[tokio::test]
async fn create_same_name() {
    let client = common::setup_client();

    let program1 = ProgramContent {
        program_name: "program1".to_string(),
        ..default_content()
    };

    let _ = client.create_program(program1.clone()).await.unwrap();
    let Error::Problem(problem) = client.create_program(program1).await.unwrap_err() else {
        unreachable!()
    };

    assert_eq!(problem.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn retrieve_all_with_filter() {
    let client = common::setup_client();

    let program1 = ProgramContent {
        program_name: "program1".to_string(),
        ..default_content()
    };
    let program2 = ProgramContent {
        program_name: "program2".to_string(),
        ..default_content()
    };
    let program3 = ProgramContent {
        program_name: "program3".to_string(),
        ..default_content()
    };

    for content in [program1, program2, program3] {
        let _ = client.create_program(content).await.unwrap();
    }

    let programs = client
        .get_programs(Filter::None, PaginationOptions { skip: 0, limit: 50 })
        .await
        .unwrap();
    assert_eq!(programs.len(), 3);

    // skip
    let programs = client
        .get_programs(Filter::None, PaginationOptions { skip: 1, limit: 50 })
        .await
        .unwrap();
    assert_eq!(programs.len(), 2);

    // limit
    let programs = client
        .get_programs(Filter::None, PaginationOptions { skip: 0, limit: 2 })
        .await
        .unwrap();
    assert_eq!(programs.len(), 2);

    // program name
    let err = client
        .get_programs(
            Filter::By(TargetLabel::Private("NONSENSE".to_string()), &[]),
            PaginationOptions { skip: 0, limit: 2 },
        )
        .await
        .unwrap_err();
    let Error::Problem(problem) = err else {
        unreachable!()
    };
    assert_eq!(problem.status, StatusCode::NOT_IMPLEMENTED);

    let programs = client
        .get_programs(
            Filter::By(TargetLabel::ProgramName, &["program1", "program2"]),
            PaginationOptions { skip: 0, limit: 50 },
        )
        .await
        .unwrap();
    assert_eq!(programs.len(), 2);
}
