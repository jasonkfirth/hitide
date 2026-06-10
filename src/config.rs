use serde_derive::Deserialize;
use std::collections::HashMap;

use config::Format;

fn default_port() -> u16 {
    4333
}

fn default_bind_address() -> std::net::IpAddr {
    std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
}

#[derive(Deserialize)]
pub struct Config {
    pub backend_host: String,

    pub frontend_url: url::Url,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_bind_address")]
    pub bind_address: std::net::IpAddr,
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        let mut src = config::Config::builder()
            .add_source(config::Environment::default())
            .add_source(config::Environment::with_prefix("HITIDE"));

        {
            let mut args = std::env::args();
            while let Some(arg) = args.next() {
                if arg == "-c" {
                    let path = args.next().expect("Missing parameter for config argument");
                    src = src.add_source(SpecificFile { path: path.into() });
                }
            }
        }

        src.build()?.try_deserialize()
    }
}

#[derive(Debug, Clone)]
struct SpecificFile {
    path: std::path::PathBuf,
}

impl config::Source for SpecificFile {
    fn clone_into_box(&self) -> Box<dyn config::Source + Send + Sync> {
        Box::new(self.clone())
    }

    fn collect(&self) -> Result<HashMap<String, config::Value>, config::ConfigError> {
        let uri = self.path.to_string_lossy().into_owned();

        let content = match std::fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(cause) => {
                return Err(config::ConfigError::FileParse {
                    uri: Some(uri),
                    cause: Box::new(cause),
                });
            }
        };

        config::FileFormat::Ini
            .parse(Some(&uri), &content)
            .map_err(|cause| config::ConfigError::FileParse {
                uri: Some(uri),
                cause,
            })
    }
}
