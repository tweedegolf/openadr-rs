mod event;
mod program;
mod report;
mod target;
mod timeline;

use std::{
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::RwLock;

use axum::body::Body;
use http_body_util::BodyExt;
use tower::{Service, ServiceExt};
use url::Url;

pub use event::*;
pub use program::*;
pub use report::*;
pub use target::*;
pub use timeline::*;

pub use crate::wire::{
    event::EventContent,
    program::{ProgramContent, ProgramId},
};
use crate::wire::{target::TargetLabel, Program};
use crate::Result;

/// Client used for interaction with a VTN.
///
/// Can be used to implement both, the VEN and the business logic
pub struct Client<T> {
    client_ref: Arc<T>,
}

pub struct ClientCredentials {
    pub client_id: String,
    client_secret: String,
    pub refresh_margin: Duration,
    pub default_credential_expires_in: Duration,
}

impl ClientCredentials {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            refresh_margin: Duration::from_secs(60),
            default_credential_expires_in: Duration::from_secs(3600),
        }
    }
}

struct AuthToken {
    token: String,
    expires_in: Duration,
    since: Instant,
}

pub struct ReqwestClientRef {
    client: reqwest::Client,
    base_url: url::Url,
    default_page_size: usize,
    auth_data: Option<ClientCredentials>,
    auth_token: RwLock<Option<AuthToken>>,
}

impl std::fmt::Debug for ReqwestClientRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ClientRef")
            .field(&self.base_url.to_string())
            .finish()
    }
}

impl ReqwestClientRef {
    /// This ensures the client is authenticated.
    ///
    /// We follow the process according to RFC 6749, section 4.4 (client
    /// credentials grant). The client id and secret are by default sent via
    /// HTTP Basic Auth.
    async fn ensure_auth(&self) -> Result<()> {
        // if there is no auth data we don't do any authentication
        let Some(auth_data) = &self.auth_data else {
            return Ok(());
        };

        // if there is a token and it is valid long enough, we don't have to do anything
        if let Some(token) = self.auth_token.read().await.as_ref() {
            if token.since.elapsed() < token.expires_in - auth_data.refresh_margin {
                return Ok(());
            }
        }

        #[derive(serde::Serialize)]
        struct AccessTokenRequest {
            grant_type: &'static str,
            #[serde(skip_serializing_if = "Option::is_none")]
            scope: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            client_id: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            client_secret: Option<String>,
        }

        // we should authenticate
        let auth_url = self.base_url.join("auth/token")?;
        let request = self.client.post(auth_url).form(&AccessTokenRequest {
            grant_type: "client_credentials",
            scope: None,
            client_id: None,
            client_secret: None,
        });
        let request = request.basic_auth(&auth_data.client_id, Some(&auth_data.client_secret));
        let request = request.header("Accept", "application/json");
        let since = Instant::now();
        let res = request.send().await?;
        if !res.status().is_success() {
            let problem = res.json::<crate::wire::oauth::OAuthError>().await?;
            return Err(crate::Error::AuthProblem(problem));
        }

        #[derive(Debug, serde::Deserialize)]
        struct AuthResult {
            access_token: String,
            token_type: String,
            #[serde(default)]
            expires_in: Option<u64>,
            // Refresh tokens aren't supported currently
            // #[serde(default)]
            // refresh_token: Option<String>,
            // #[serde(default)]
            // scope: Option<String>,
            // #[serde(flatten)]
            // other: std::collections::HashMap<String, serde_json::Value>,
        }

        let auth_result = res.json::<AuthResult>().await?;
        if auth_result.token_type.to_lowercase() != "bearer" {
            return Err(crate::Error::OAuthTokenNotBearer);
        }
        let token = AuthToken {
            token: auth_result.access_token,
            expires_in: auth_result
                .expires_in
                .map(Duration::from_secs)
                .unwrap_or(auth_data.default_credential_expires_in),
            since,
        };

        *self.auth_token.write().await = Some(token);
        Ok(())
    }
}

pub trait ClientRef {
    fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> impl Future<Output = Result<T>> + Send;

    fn post<S, T>(
        &self,
        path: &str,
        body: &S,
        query: &[(&str, &str)],
    ) -> impl Future<Output = Result<T>> + Send
    where
        S: serde::ser::Serialize + Sync,
        T: serde::de::DeserializeOwned;

    fn put<S, T>(
        &self,
        path: &str,
        body: &S,
        query: &[(&str, &str)],
    ) -> impl Future<Output = Result<T>> + Send
    where
        S: serde::ser::Serialize + Sync,
        T: serde::de::DeserializeOwned;

    fn delete(&self, path: &str, query: &[(&str, &str)])
        -> impl Future<Output = Result<()>> + Send;

    fn default_page_size(&self) -> usize;
}

impl ReqwestClientRef {
    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        mut request: reqwest::RequestBuilder,
        query: &[(&str, &str)],
    ) -> Result<T> {
        self.ensure_auth().await?;
        request = request.header("Accept", "application/json");
        if !query.is_empty() {
            request = request.query(&query);
        }

        // read token and insert in request if available
        {
            let token = self.auth_token.read().await;
            if let Some(token) = token.as_ref() {
                request = request.bearer_auth(&token.token);
            }
        }
        let res = request.send().await?;

        // handle any errors returned by the server
        if !res.status().is_success() {
            let problem = res.json::<crate::wire::Problem>().await?;
            return Err(crate::Error::from(problem));
        }

        Ok(res.json().await?)
    }
}

impl ClientRef for ReqwestClientRef {
    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T> {
        let url = self.base_url.join(path)?;
        let request = self.client.get(url);
        self.request(request, query).await
    }

    async fn post<S, T>(&self, path: &str, body: &S, query: &[(&str, &str)]) -> Result<T>
    where
        S: serde::ser::Serialize + Sync,
        T: serde::de::DeserializeOwned,
    {
        let url = self.base_url.join(path)?;
        let request = self.client.post(url).json(body);
        self.request(request, query).await
    }

    async fn put<S, T>(&self, path: &str, body: &S, query: &[(&str, &str)]) -> Result<T>
    where
        S: serde::ser::Serialize + Sync,
        T: serde::de::DeserializeOwned,
    {
        let url = self.base_url.join(path)?;
        let request = self.client.put(url).json(body);
        self.request(request, query).await
    }

    async fn delete(&self, path: &str, query: &[(&str, &str)]) -> Result<()> {
        let url = self.base_url.join(path)?;
        let request = self.client.delete(url);
        self.request(request, query).await
    }

    fn default_page_size(&self) -> usize {
        self.default_page_size
    }
}

pub struct MockClientRef {
    router: Arc<tokio::sync::Mutex<axum::Router>>,
    default_page_size: usize,
}

impl MockClientRef {
    pub fn new(router: axum::Router) -> Self {
        MockClientRef {
            router: Arc::new(tokio::sync::Mutex::new(router)),
            default_page_size: 50,
        }
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        method: axum::http::Method,
        path: &str,
        body: Option<Vec<u8>>,
        query: &[(&str, &str)],
    ) -> Result<T> {
        let mut uri = format!("/{path}?");
        let mut it = query.iter().peekable();

        while let Some((key, value)) = it.next() {
            uri.push_str(key);
            uri.push('=');
            uri.push_str(value);

            if it.peek().is_some() {
                uri.push('&');
            }
        }

        let request = axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::from(body.unwrap_or_default()))
            .unwrap();

        let response =
            ServiceExt::<axum::http::Request<Body>>::ready(&mut *self.router.lock().await)
                .await
                .unwrap()
                .call(request)
                .await
                .unwrap();

        let body = response.into_body().collect().await.unwrap().to_bytes();

        Ok(serde_json::from_slice(&body).unwrap())
    }
}

impl ClientRef for MockClientRef {
    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T> {
        self.request(axum::http::Method::GET, path, None, query)
            .await
    }

    async fn post<S, T>(&self, path: &str, body: &S, query: &[(&str, &str)]) -> Result<T>
    where
        S: serde::ser::Serialize + Sync,
        T: serde::de::DeserializeOwned,
    {
        let body = serde_json::to_vec(body)?;
        self.request(axum::http::Method::POST, path, Some(body), query)
            .await
    }

    async fn put<S, T>(&self, path: &str, body: &S, query: &[(&str, &str)]) -> Result<T>
    where
        S: serde::ser::Serialize + Sync,
        T: serde::de::DeserializeOwned,
    {
        let body = serde_json::to_vec(body)?;
        self.request(axum::http::Method::PUT, path, Some(body), query)
            .await
    }

    async fn delete(&self, path: &str, query: &[(&str, &str)]) -> Result<()> {
        self.request(axum::http::Method::DELETE, path, None, query)
            .await
    }

    fn default_page_size(&self) -> usize {
        self.default_page_size
    }
}

impl Client<ReqwestClientRef> {
    /// Create a new client for a VTN located at the specified URL
    pub fn with_url(base_url: Url, auth: Option<ClientCredentials>) -> Self {
        let client = reqwest::Client::new();
        Self::with_reqwest(base_url, client, auth)
    }

    /// Create a new client, but use the specific reqwest client instead of
    /// the default one. This allows you to configure proxy settings, timeouts, etc.
    pub fn with_reqwest(
        base_url: Url,
        client: reqwest::Client,
        auth: Option<ClientCredentials>,
    ) -> Self {
        let client_ref = ReqwestClientRef {
            client,
            base_url,
            default_page_size: 50,
            auth_data: auth,
            auth_token: RwLock::new(None),
        };

        Self::new(client_ref)
    }
}

impl<C: ClientRef> Client<C> {
    pub fn new(client_ref: C) -> Self {
        Client {
            client_ref: Arc::new(client_ref),
        }
    }

    /// Create a new program on the VTN
    pub async fn create_program(&self, program_data: ProgramContent) -> Result<ProgramClient<C>> {
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
    ) -> Result<Vec<ProgramClient<C>>> {
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
            query.push(("targetType", target_type_ref.as_str()));
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
    pub async fn get_program(&self, target: Target<'_>) -> Result<ProgramClient<C>> {
        let mut programs = self
            .get_programs_req(Some(target.target_label()), target.target_values(), 0, 2)
            .await?;

        match programs[..] {
            [] => Err(crate::Error::ObjectNotFound),
            [_] => Ok(programs.remove(0)),
            [..] => Err(crate::Error::DuplicateObject),
        }
    }

    /// Get a list of programs from the VTN with the given query parameters
    pub async fn get_program_list(&self, target: Target<'_>) -> Result<Vec<ProgramClient<C>>> {
        let page_size = self.client_ref.default_page_size();
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
    pub async fn get_all_programs(&self) -> Result<Vec<ProgramClient<C>>> {
        let page_size = self.client_ref.default_page_size();
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
    pub async fn get_program_by_name(&self, name: &str) -> Result<ProgramClient<C>> {
        self.get_program(Target::Program(name)).await
    }

    /// Get a program by id
    pub async fn get_program_by_id(&self, id: &ProgramId) -> Result<ProgramClient<C>> {
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
