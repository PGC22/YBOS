use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Limited};
use hyper::header::{LOCATION, USER_AGENT};
use hyper::{Request, Uri};
use hyper_tls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::time::Duration;
use thiserror::Error;

const USER_AGENT_STR: &str = "YBOS/0.1 (+https://github.com/PGC22/YBOS)";
const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10MB
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_REDIRECTS: usize = 5;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("TLS error: {0}")]
    Tls(String),
    #[error("HTTP status error: {0}")]
    Status(u16),
    #[error("Body error: {0}")]
    Body(String),
    #[error("Timeout error")]
    Timeout,
    #[error("Too many redirects")]
    TooManyRedirects,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn get(&self, url: &str) -> Result<HttpResponse, HttpError>;
}

pub struct HyperHttpClient {
    client: Client<HttpsConnector<HttpConnector>, Empty<Bytes>>,
}

impl HyperHttpClient {
    pub fn new() -> Self {
        let https = HttpsConnector::new();
        let client = Client::builder(TokioExecutor::new()).build(https);
        Self { client }
    }
}

impl Default for HyperHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpClient for HyperHttpClient {
    async fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
        let mut current_url = url.to_string();

        for _ in 0..MAX_REDIRECTS {
            let uri: Uri = current_url.parse().map_err(|_| HttpError::InvalidUrl(current_url.clone()))?;

            let req = Request::builder()
                .uri(&uri)
                .header(USER_AGENT, USER_AGENT_STR)
                .body(Empty::<Bytes>::new())
                .map_err(|e| HttpError::Network(e.to_string()))?;

            let res_fut = self.client.request(req);
            let res = tokio::time::timeout(DEFAULT_TIMEOUT, res_fut)
                .await
                .map_err(|_| HttpError::Timeout)?
                .map_err(|e| {
                    let err_str = e.to_string();
                    if err_str.contains("TLS") || err_str.contains("certificate") {
                        HttpError::Tls(err_str)
                    } else {
                        HttpError::Network(err_str)
                    }
                })?;

            let status = res.status();
            if status.is_redirection() {
                if let Some(location) = res.headers().get(LOCATION) {
                    let location_str = location.to_str().map_err(|_| HttpError::Network("Invalid location header".to_string()))?;

                    if location_str.starts_with("//") {
                        let scheme = uri.scheme_str().unwrap_or("http");
                        current_url = format!("{}:{}", scheme, location_str);
                    } else if location_str.starts_with('/') {
                        let scheme = uri.scheme_str().unwrap_or("http");
                        let authority = uri.authority().map(|a| a.as_str()).unwrap_or("");
                        current_url = format!("{}://{}{}", scheme, authority, location_str);
                    } else if !location_str.starts_with("http") {
                        // Handle relative path without leading slash (uncommon but possible)
                        let scheme = uri.scheme_str().unwrap_or("http");
                        let authority = uri.authority().map(|a| a.as_str()).unwrap_or("");
                        let path = uri.path();
                        let dir = match path.rfind('/') {
                            Some(idx) => &path[..idx + 1],
                            None => "/",
                        };
                        current_url = format!("{}://{}{}{}", scheme, authority, dir, location_str);
                    } else {
                        current_url = location_str.to_string();
                    }
                    continue;
                }
            }

            let headers = res.headers().iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            let body = res.into_body();
            let limited_body = Limited::new(body, MAX_BODY_SIZE);

            let collected = limited_body.collect().await
                .map_err(|e| {
                    if e.downcast_ref::<http_body_util::LengthLimitError>().is_some() {
                        HttpError::Body("Body too large".to_string())
                    } else {
                        HttpError::Body(e.to_string())
                    }
                })?;

            return Ok(HttpResponse {
                status: status.as_u16(),
                headers,
                body: collected.to_bytes().to_vec(),
            });
        }

        Err(HttpError::TooManyRedirects)
    }
}

pub struct MockHttpClient {
    responses: Vec<(String, HttpResponse)>,
}

impl MockHttpClient {
    pub fn new(responses: Vec<(String, HttpResponse)>) -> Self {
        Self { responses }
    }
}

#[async_trait]
impl HttpClient for MockHttpClient {
    async fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
        for (pattern, resp) in &self.responses {
            if url.contains(pattern) {
                return Ok(resp.clone());
            }
        }
        Err(HttpError::Network(format!("No mock for {}", url)))
    }
}
