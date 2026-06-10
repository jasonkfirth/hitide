#![allow(unused_braces)]
#![allow(dead_code)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::result_large_err)]
#![allow(clippy::too_many_arguments)]

use crate::resp_types::RespLoginInfo;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use trout::http02::RoutingFailureExtHttp;

mod components;
mod config;
mod hyper;
mod lang;
mod query_types;
mod resp_types;
mod routes;
mod util;

pub use lang::Translator;

use self::components::HTPage;
use self::config::Config;

#[derive(Deserialize, Serialize, Eq, PartialEq, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub enum SortType {
    Hot,
    New,
    Top,
}

impl SortType {
    pub const VALUES: &'static [SortType] = &[SortType::Hot, SortType::New, SortType::Top];

    pub fn as_str(&self) -> &'static str {
        match self {
            SortType::Hot => "hot",
            SortType::New => "new",
            SortType::Top => "top",
        }
    }

    pub fn lang_key(&self) -> lang::LangKey<'static> {
        match self {
            SortType::Hot => lang::sort_hot(),
            SortType::New => lang::sort_new(),
            SortType::Top => lang::sort_top(),
        }
    }
}

#[derive(Clone)]
pub struct HttpClient {
    inner: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
}

impl HttpClient {
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
    const STREAMING_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

    pub fn new() -> Self {
        Self {
            inner: hyper::Client::builder().build(hyper_tls::HttpsConnector::new()),
        }
    }

    pub async fn request(
        &self,
        request: hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<hyper::Body>, crate::Error> {
        /*
            Requests with an unknown request-body length are usually browser
            uploads being streamed to Lotide. Give those requests a larger
            envelope so slow but active uploads are not killed by the normal
            backend response timeout.
        */
        let timeout = if request.body().size_hint().upper().is_none() {
            Self::STREAMING_REQUEST_TIMEOUT
        } else {
            Self::REQUEST_TIMEOUT
        };

        tokio::time::timeout(timeout, self.inner.request(request))
            .await
            .map_err(|_| {
                crate::Error::BackendUnavailable(format!(
                    "Backend API request timed out after {:?}",
                    timeout
                ))
            })?
            .map_err(|err| crate::Error::BackendUnavailable(err.to_string()))
    }

    pub async fn get(&self, uri: hyper::Uri) -> Result<hyper::Response<hyper::Body>, crate::Error> {
        self.request(hyper::Request::get(uri).body(Default::default())?)
            .await
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

const HTTP_BODY_MAX_BYTES: usize = 32 * 1024 * 1024;

async fn read_body_limited(mut body: hyper::Body, limit: usize) -> Result<Vec<u8>, Error> {
    /*
        Hitide reads both browser form bodies and backend API responses through
        this helper. Keeping the limit central prevents one slow or oversized
        request from turning into unbounded process memory.
    */
    if let Some(upper) = body.size_hint().upper()
        && upper > limit as u64
    {
        return Err(Error::InternalStr(format!(
            "HTTP body exceeded {limit} byte limit"
        )));
    }

    let mut data = Vec::new();
    while let Some(chunk) = body.data().await {
        let chunk = chunk.map_err(|err| Error::BackendUnavailable(err.to_string()))?;
        if data.len().saturating_add(chunk.len()) > limit {
            return Err(Error::InternalStr(format!(
                "HTTP body exceeded {limit} byte limit"
            )));
        }

        data.extend_from_slice(&chunk);
    }

    Ok(data)
}

pub async fn read_body_with_timeout(body: hyper::Body) -> Result<Vec<u8>, Error> {
    let bytes = tokio::time::timeout(
        HttpClient::REQUEST_TIMEOUT,
        read_body_limited(body, HTTP_BODY_MAX_BYTES),
    )
    .await
    .map_err(|_| {
        crate::Error::BackendUnavailable(format!(
            "Backend API response body read timed out after {:?}",
            HttpClient::REQUEST_TIMEOUT
        ))
    })??;

    Ok(bytes)
}

fn response_for_route_error(err: Error) -> hyper::Response<hyper::Body> {
    /*
        Keep the public error text useful. Backend reachability and backend 5xx
        failures point at Lotide; unexpected local render or routing errors point
        at Hitide. Detailed diagnostics stay in the logs.

        This path cannot rely on the backend for page settings, since the backend
        might be down. It uses a small local fallback identity while rendering
        the same HTML shell and CSS as other error pages.
    */
    match err {
        Error::UserError(res) => res,
        Error::RoutingError(err) => {
            let res: hyper::Response<hyper::Body> = err.to_simple_response();
            themed_error_response(
                res.status(),
                res.status().canonical_reason().unwrap_or("Not found"),
                "That page was not found.",
            )
        }
        Error::RemoteError((status, message)) if status.is_client_error() => themed_error_response(
            status,
            status.canonical_reason().unwrap_or("Request problem"),
            message,
        ),
        Error::RemoteError((status, message)) if status.is_server_error() => {
            log::warn!("Backend API returned {status}: {message}");
            themed_error_response(
                hyper::StatusCode::BAD_GATEWAY,
                "No backend",
                "Lotide returned an internal error while Hitide was rendering this page.",
            )
        }
        Error::BackendUnavailable(message) => {
            log::warn!("Backend API unavailable: {message}");

            if message.starts_with("Backend API request timed out after")
                || message.starts_with("Backend API response body read timed out after")
            {
                themed_error_response(
                    hyper::StatusCode::GATEWAY_TIMEOUT,
                    "No backend",
                    "Hitide timed out while waiting for Lotide.",
                )
            } else {
                themed_error_response(
                    hyper::StatusCode::BAD_GATEWAY,
                    "No backend",
                    "Hitide could not reach Lotide.",
                )
            }
        }
        err => {
            log::error!("Error: {err:?}");

            themed_error_response(
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                "No frontend",
                "Hitide hit an internal error while rendering this page.",
            )
        }
    }
}

pub struct RouteContext {
    backend_host: String,
    frontend_url: url::Url,
    http_client: HttpClient,
}

pub type RouteNode<P> = trout::Node<
    P,
    hyper::Request<hyper::Body>,
    std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<hyper::Response<hyper::Body>, Error>> + Send>,
    >,
    Arc<RouteContext>,
>;

#[derive(Debug)]
pub enum Error {
    Internal(Box<dyn std::error::Error + Send>),
    InternalStr(String),
    InternalStrStatic(&'static str),
    UserError(hyper::Response<hyper::Body>),
    RoutingError(trout::RoutingFailure),
    RemoteError((hyper::StatusCode, String)),
    BackendUnavailable(String),
    InternalUserError(String),
}

impl<T: 'static + std::error::Error + Send> From<T> for Error {
    fn from(err: T) -> Error {
        Error::Internal(Box::new(err))
    }
}

#[derive(Debug)]
pub struct PageBaseData {
    pub login: Option<RespLoginInfo>,
    pub site_name: String,
    pub site_logo_url: Option<String>,
    pub site_css_url: Option<String>,
}

impl PageBaseData {
    pub fn is_site_admin(&self) -> bool {
        match &self.login {
            None => false,
            Some(login) => login.user.is_site_admin,
        }
    }
}

pub fn simple_response(
    code: hyper::StatusCode,
    text: impl Into<hyper::Body>,
) -> hyper::Response<hyper::Body> {
    let mut res = hyper::Response::new(text.into());
    *res.status_mut() = code;
    res
}

fn themed_error_response(
    code: hyper::StatusCode,
    title: impl Into<String>,
    message: impl Into<String>,
) -> hyper::Response<hyper::Body> {
    let title = title.into();
    let message = message.into();
    let status_text = code.canonical_reason().map_or_else(
        || code.as_u16().to_string(),
        |reason| format!("{} {}", code.as_u16(), reason),
    );
    let lang = get_lang_for_headers(&hyper::HeaderMap::new());
    let base_data = PageBaseData {
        login: None,
        site_name: "lotide".to_owned(),
        site_logo_url: None,
        site_css_url: None,
    };

    let mut res = hyper::Response::new(
        render::html! {
            <HTPage base_data={&base_data} lang={&lang} title={title.as_str()}>
                <section class={"errorPage"} aria-labelledby={"errorPageTitle"}>
                    <p class={"errorStatus"}>{status_text.as_str()}</p>
                    <h1 id={"errorPageTitle"}>{title.as_str()}</h1>
                    <p class={"errorMessage"}>{message.as_str()}</p>
                    <div class={"actionList"}>
                        <a href={"/"}>{"Home"}</a>
                        <a href={"/communities"}>{"Communities"}</a>
                    </div>
                </section>
            </HTPage>
        }
        .into(),
    );
    *res.status_mut() = code;
    res.headers_mut().insert(
        hyper::header::CONTENT_TYPE,
        hyper::header::HeaderValue::from_static("text/html"),
    );
    res
}

lazy_static::lazy_static! {
    static ref LANG_MAP: HashMap<unic_langid::LanguageIdentifier, fluent::FluentResource> = {
        let mut result = HashMap::new();

        result.insert(unic_langid::langid!("de"), fluent::FluentResource::try_new(include_str!("../res/lang/de.ftl").to_owned()).expect("Failed to parse translation"));
        result.insert(unic_langid::langid!("en"), fluent::FluentResource::try_new(include_str!("../res/lang/en.ftl").to_owned()).expect("Failed to parse translation"));
        result.insert(unic_langid::langid!("eo"), fluent::FluentResource::try_new(include_str!("../res/lang/eo.ftl").to_owned()).expect("Failed to parse translation"));
        result.insert(unic_langid::langid!("fr"), fluent::FluentResource::try_new(include_str!("../res/lang/fr.ftl").to_owned()).expect("Failed to parse translation"));
        result.insert(unic_langid::langid!("fa"), fluent::FluentResource::try_new(include_str!("../res/lang/fa.ftl").to_owned()).expect("Failed to parse translation"));

        result
    };

    static ref LANGS: Vec<unic_langid::LanguageIdentifier> = {
        LANG_MAP.keys().cloned().collect()
    };
}

pub fn get_lang_for_headers(headers: &hyper::header::HeaderMap) -> Translator {
    let default = unic_langid::langid!("en");
    let languages = match headers
        .get(hyper::header::ACCEPT_LANGUAGE)
        .and_then(|x| x.to_str().ok())
    {
        Some(accept_language) => {
            let requested = fluent_langneg::accepted_languages::parse(accept_language);
            fluent_langneg::negotiate_languages(
                &requested,
                &LANGS,
                Some(&default),
                fluent_langneg::NegotiationStrategy::Filtering,
            )
        }
        None => vec![&default],
    };

    let mut bundle = fluent::bundle::FluentBundle::<
        &'static fluent::FluentResource,
        intl_memoizer::concurrent::IntlLangMemoizer,
    >::new_concurrent(languages.iter().copied().cloned().collect());
    for lang in &languages {
        if let Err(errors) = bundle.add_resource(&LANG_MAP[lang]) {
            for err in errors {
                if let fluent::FluentError::Overriding { .. } = err {
                } else {
                    log::error!("Failed to add language resource: {err:?}");
                    break;
                }
            }
        }
    }

    Translator::new(bundle, languages[0].clone())
}

pub fn get_lang_for_req(req: &hyper::Request<hyper::Body>) -> Translator {
    get_lang_for_headers(req.headers())
}

pub fn bool_as_str(src: bool) -> &'static str {
    if src { "true" } else { "false" }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let config = Config::load().expect("Failed to load config");
    let listen_addr = std::net::SocketAddr::new(config.bind_address, config.port);

    let routes = Arc::new(routes::route_root());
    let context = Arc::new(RouteContext {
        backend_host: config.backend_host,
        frontend_url: config.frontend_url,
        http_client: HttpClient::new(),
    });

    log::info!("Listening on {listen_addr}");

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let routes = routes.clone();
        let context = context.clone();

        tokio::spawn(async move {
            let io = hyper::rt::TokioIo::new(stream);
            let service =
                hyper::service::service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                    let req = req.map(hyper::Body::from_incoming);
                    let routes = routes.clone();
                    let context = context.clone();

                    async move {
                        let result = match routes.route(req, context) {
                            Ok(fut) => fut.await,
                            Err(err) => Err(Error::RoutingError(err)),
                        };

                        Ok::<_, std::convert::Infallible>(match result {
                            Ok(val) => val,
                            Err(err) => response_for_route_error(err),
                        })
                    }
                });

            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service)
                .await
            {
                log::warn!("HTTP connection failed: {err}");
            }
        });
    }

    #[allow(unreachable_code)]
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::hyper;

    async fn response_text(response: hyper::Response<hyper::Body>) -> String {
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn bounded_body_reader_accepts_small_bodies() {
        let body = super::read_body_limited(hyper::Body::from("small"), 8)
            .await
            .unwrap();

        assert_eq!(body, b"small");
    }

    #[tokio::test]
    async fn bounded_body_reader_rejects_oversized_bodies() {
        let err = super::read_body_limited(hyper::Body::from("large"), 4)
            .await
            .unwrap_err();

        match err {
            super::Error::InternalStr(message) => {
                assert!(message.contains("HTTP body exceeded 4 byte limit"));
            }
            err => panic!("unexpected error: {:?}", err),
        }
    }

    fn assert_themed_error_headers(response: &hyper::Response<hyper::Body>) {
        assert_eq!(
            response.headers().get(hyper::header::CONTENT_TYPE).unwrap(),
            "text/html"
        );
    }

    async fn assert_themed_error_body(response: hyper::Response<hyper::Body>, expected: &str) {
        let body = response_text(response).await;

        assert!(body.contains(expected), "missing expected text: {}", body);
        assert!(body.contains("mainHeader"), "missing site header: {}", body);
        assert!(
            body.contains("errorPage"),
            "missing error page class: {}",
            body
        );
    }

    #[tokio::test]
    async fn backend_client_errors_pass_through_to_browser() {
        let response = super::response_for_route_error(super::Error::RemoteError((
            hyper::StatusCode::NOT_FOUND,
            "No such post".to_owned(),
        )));

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);
        assert_themed_error_headers(&response);
        assert_themed_error_body(response, "No such post").await;
    }

    #[tokio::test]
    async fn backend_server_errors_are_reported_as_no_backend() {
        let response = super::response_for_route_error(super::Error::RemoteError((
            hyper::StatusCode::BAD_GATEWAY,
            "upstream failed".to_owned(),
        )));

        assert_eq!(response.status(), hyper::StatusCode::BAD_GATEWAY);
        assert_themed_error_headers(&response);
        assert_themed_error_body(response, "No backend").await;
    }

    #[tokio::test]
    async fn backend_timeouts_become_no_backend_gateway_timeouts() {
        for message in [
            "Backend API request timed out after 20s",
            "Backend API response body read timed out after 20s",
        ] {
            let response = super::response_for_route_error(super::Error::BackendUnavailable(
                message.to_owned(),
            ));

            assert_eq!(response.status(), hyper::StatusCode::GATEWAY_TIMEOUT);
            assert_themed_error_headers(&response);
            assert_themed_error_body(response, "No backend").await;
        }
    }

    #[tokio::test]
    async fn backend_connection_errors_become_no_backend() {
        let response = super::response_for_route_error(super::Error::BackendUnavailable(
            "connection refused".to_owned(),
        ));

        assert_eq!(response.status(), hyper::StatusCode::BAD_GATEWAY);
        assert_themed_error_headers(&response);
        assert_themed_error_body(response, "No backend").await;
    }

    #[tokio::test]
    async fn frontend_internal_errors_are_reported_as_no_frontend() {
        let response = super::response_for_route_error(super::Error::InternalStr(
            "template failed".to_owned(),
        ));

        assert_eq!(response.status(), hyper::StatusCode::INTERNAL_SERVER_ERROR);
        assert_themed_error_headers(&response);
        assert_themed_error_body(response, "No frontend").await;
    }
}
