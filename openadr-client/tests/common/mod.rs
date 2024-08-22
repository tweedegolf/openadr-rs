use std::env::VarError;

use openadr_client::{Client, ClientCredentials, MockClientRef, ProgramClient};
use openadr_vtn::data_source::AuthInfo;
use openadr_wire::program::ProgramContent;
use url::Url;

pub fn setup_mock_client() -> Client {
    use openadr_vtn::{data_source::InMemoryStorage, jwt::JwtManager, state::AppState};

    let auth_info = AuthInfo::bl_admin();
    let client_credentials = ClientCredentials::admin();

    let storage = InMemoryStorage::default();
    storage.auth.try_write().unwrap().push(auth_info);

    let app_state = AppState::new(storage, JwtManager::from_secret(b"test"));

    MockClientRef::new(app_state.into_router()).into_client(Some(client_credentials))
}

pub fn setup_url_client(url: Url) -> Client {
    Client::with_url(url, Some(ClientCredentials::admin()))
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

#[allow(unused)]
pub async fn setup_program_client(program_name: impl ToString) -> ProgramClient {
    let client = setup_client();

    let program_content = ProgramContent {
        object_type: None,
        program_name: program_name.to_string(),
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
    };

    client.create_program(program_content).await.unwrap()
}
