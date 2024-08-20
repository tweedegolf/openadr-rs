use openadr_wire::program::ProgramContent;

mod common;

#[tokio::test]
async fn basic_create_read() -> Result<(), openadr_client::Error> {
    let client = common::setup_client();

    client
        .create_program(ProgramContent::new("test-prog"))
        .await?;

    let programs = client.get_all_programs().await?;
    assert_eq!(programs.len(), 1);
    assert_eq!(programs[0].content().program_name, "test-prog");

    Ok(())
}
