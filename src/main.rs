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
    const UPLOAD_REQUEST_TIMEOUT: Duration = Duration::from_secs(5 * 60);

    pub fn new() -> Self {
        Self {
            inner: hyper::Client::builder().build(hyper_tls::HttpsConnector::new()),
        }
    }

    pub async fn request(
        &self,
        request: hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<hyper::Body>, crate::Error> {
        self.request_with_timeout(request, Self::REQUEST_TIMEOUT)
            .await
    }

    pub async fn request_upload(
        &self,
        request: hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<hyper::Body>, crate::Error> {
        self.request_with_timeout(request, Self::UPLOAD_REQUEST_TIMEOUT)
            .await
    }

    async fn request_with_timeout(
        &self,
        request: hyper::Request<hyper::Body>,
        timeout: Duration,
    ) -> Result<hyper::Response<hyper::Body>, crate::Error> {
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
        self.request(hyper::Request::get(uri).body(hyper::Body::default())?)
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
        Keep the public error text actionable. Backend reachability and backend
        5xx failures point at Lotide; unexpected local render or routing errors
        point at Hitide. Detailed diagnostics stay in the logs.

        This path cannot rely on the backend for page settings, since the most
        common failure is that the backend is unavailable. The themed error page
        therefore uses a small local fallback identity while still rendering the
        same HTML shell and CSS as the rest of the site.
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
    pub fn fallback(login: Option<RespLoginInfo>) -> Self {
        Self {
            login,
            site_name: "lotide".to_owned(),
            site_logo_url: None,
            site_css_url: None,
        }
    }

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

pub fn parse_query_string<'de, T>(query: Option<&'de str>) -> Result<T, Error>
where
    T: serde::Deserialize<'de>,
{
    /*
        Browser query strings are public input. A malformed enum or number is a
        bad request, not a frontend failure, and logging the original value can
        turn simple scanner noise into very large log lines.
    */
    serde_urlencoded::from_str(query.unwrap_or("")).map_err(|_| {
        Error::UserError(simple_response(
            hyper::StatusCode::BAD_REQUEST,
            "Invalid query string.",
        ))
    })
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
    let base_data = PageBaseData::fallback(None);

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

#[derive(Debug, Eq, PartialEq)]
struct CliArgs {
    config_file_path: Option<std::ffi::OsString>,
}

fn help_text(program_name: &str) -> String {
    /*
        Hitide has intentionally simple process management. Keep the help text
        plain enough for service files and shell scripts while still spelling
        out the backend/frontend split that trips up new installs.
    */
    format!(
        "\
hitide {version}

Server-rendered web frontend for Lotide.

Usage:
  {program_name} [OPTIONS]

Options:
  -c, --config <FILE>    Read configuration from an INI file
  -h, --help             Print this help text
  -V, --version          Print version information

Configuration:
  BACKEND_HOST           Lotide backend URL, for example http://127.0.0.1:3333
  FRONTEND_URL           Public Hitide URL, for example https://example.com
  PORT                   TCP port to listen on, default 4333
  BIND_ADDRESS           Listen address, default 127.0.0.1
  HITIDE_*               Any setting can also be provided with this prefix

Examples:
  {program_name} -c /etc/hitide.ini
  BACKEND_HOST=http://127.0.0.1:3333 FRONTEND_URL=https://example.com {program_name}
",
        version = env!("CARGO_PKG_VERSION"),
    )
}

fn version_text() -> String {
    format!("hitide {}", env!("CARGO_PKG_VERSION"))
}

fn parse_cli_args_from<I, S>(args: I) -> Result<Option<CliArgs>, String>
where
    I: IntoIterator<Item = S>,
    S: Into<std::ffi::OsString>,
{
    let mut args = args.into_iter();
    let _program_name = args.next();
    let mut config_file_path = None;

    while let Some(arg) = args.next() {
        let arg = arg.into();
        let Some(arg_str) = arg.to_str() else {
            return Err("Command-line arguments must be valid Unicode".to_owned());
        };

        match arg_str {
            "-h" | "--help" => {
                println!("{}", help_text("hitide"));
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("{}", version_text());
                return Ok(None);
            }
            "-c" | "--config" => {
                let Some(path) = args.next() else {
                    return Err(format!("Missing value for {arg_str}"));
                };

                config_file_path = Some(path.into());
            }
            _ if arg_str.starts_with("--config=") => {
                let value = arg_str.trim_start_matches("--config=");

                if value.is_empty() {
                    return Err("Missing value for --config".to_owned());
                }

                config_file_path = Some(value.into());
            }
            _ => return Err(format!("Unknown option: {arg_str}")),
        }
    }

    Ok(Some(CliArgs { config_file_path }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let Some(cli_args) = parse_cli_args_from(std::env::args_os()).map_err(|message| {
        eprintln!("{message}\n\n{}", help_text("hitide"));
        std::io::Error::new(std::io::ErrorKind::InvalidInput, message)
    })?
    else {
        return Ok(());
    };
    let config = Config::load(cli_args.config_file_path.as_deref()).expect("Failed to load config");
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

    #[test]
    fn help_text_documents_frontend_runtime_settings() {
        let help = super::help_text("hitide");

        assert!(help.contains("Server-rendered web frontend for Lotide"));
        assert!(help.contains("-c, --config <FILE>"));
        assert!(help.contains("BACKEND_HOST"));
        assert!(help.contains("FRONTEND_URL"));
        assert!(help.contains("BIND_ADDRESS"));
        assert!(help.contains("HITIDE_*"));
    }

    #[test]
    fn cli_parser_accepts_config_options() {
        let args = super::parse_cli_args_from(["hitide", "-c", "/etc/hitide.ini"])
            .unwrap()
            .unwrap();

        assert_eq!(
            args.config_file_path.as_deref(),
            Some(std::ffi::OsStr::new("/etc/hitide.ini"))
        );

        let args = super::parse_cli_args_from(["hitide", "--config=/etc/hitide.ini"])
            .unwrap()
            .unwrap();

        assert_eq!(
            args.config_file_path.as_deref(),
            Some(std::ffi::OsStr::new("/etc/hitide.ini"))
        );
    }

    #[test]
    fn cli_parser_rejects_missing_config_value() {
        let err = super::parse_cli_args_from(["hitide", "--config"]).unwrap_err();

        assert!(err.contains("Missing value for --config"));
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

    #[test]
    fn query_parser_rejects_invalid_public_input_as_bad_request() {
        #[derive(Debug, serde_derive::Deserialize)]
        struct Query {
            sort: super::SortType,
        }

        let err = super::parse_query_string::<Query>(Some("sort=not_a_sort")).unwrap_err();

        match err {
            super::Error::UserError(response) => {
                assert_eq!(response.status(), hyper::StatusCode::BAD_REQUEST);
            }
            err => panic!("unexpected error: {:?}", err),
        }
    }

    #[test]
    fn query_parser_accepts_valid_public_input() {
        #[derive(Debug, serde_derive::Deserialize)]
        struct Query {
            sort: super::SortType,
        }

        let query = super::parse_query_string::<Query>(Some("sort=new")).unwrap();

        assert_eq!(query.sort, super::SortType::New);
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

    #[test]
    fn upload_requests_have_a_longer_timeout_than_page_requests() {
        assert!(super::HttpClient::UPLOAD_REQUEST_TIMEOUT > super::HttpClient::REQUEST_TIMEOUT);
        assert_eq!(
            super::HttpClient::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(20)
        );
        assert_eq!(
            super::HttpClient::UPLOAD_REQUEST_TIMEOUT,
            std::time::Duration::from_secs(5 * 60)
        );
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
