use openadr_wire::program::ProgramContent;

mod helper {
    use std::env::VarError;

    use openadr_client::{Client, ClientCredentials, MockClientRef};
    use openadr_vtn::data_source::AuthInfo;
    use url::Url;

    pub fn setup_mock_client() -> Client {
        use openadr_vtn::{data_source::InMemoryStorage, jwt::JwtManager, state::AppState};

        let storage = InMemoryStorage::default();
        storage.auth.try_write().unwrap().push(AuthInfo {
            client_id: "admin".into(),
            client_secret: "admin".into(),
            role: openadr_vtn::jwt::AuthRole::BL,
            ven: None,
        });

        let app_state = AppState::new(storage, JwtManager::from_secret(b"test"));

        MockClientRef::new(app_state.into_router())
            .into_client(Some(ClientCredentials::new("admin".into(), "admin".into())))
    }

    pub fn setup_url_client(url: Url) -> Client {
        Client::with_url(
            url,
            Some(ClientCredentials::new("admin".into(), "admin".into())),
        )
    }

    pub fn setup_client() -> Client {
        match std::env::var("OPENADR_RS_VTN_URL") {
            Ok(url) => match url.parse() {
                Ok(url) => setup_url_client(url),
                Err(e) => panic!("Could not parse URL: {e}"),
            },
            Err(VarError::NotPresent) => setup_mock_client(),
            Err(VarError::NotUnicode(e)) => panic!("Could not parse URL: {e:?}"),
        }
    }
}

#[tokio::test]
async fn basic_create_read() -> Result<(), openadr_client::Error> {
    let client = helper::setup_client();

    client
        .create_program(ProgramContent::new("test-prog"))
        .await?;

    let programs = client.get_all_programs().await?;
    assert_eq!(programs.len(), 1);
    assert_eq!(programs[0].data().program_name, "test-prog");

    Ok(())
}
