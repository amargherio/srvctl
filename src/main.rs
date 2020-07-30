use anyhow::anyhow;
use clap::{App, Arg, ArgMatches, Clap};
use log::{debug, error, info, trace, warn};
use serde::Deserialize;

use srvctl::dns::resolve_srv;

use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ControllerConfig {
    domains: Vec<String>,
    #[serde(alias = "createService")]
    create_service: bool,
}

impl std::fmt::Display for ControllerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UNIMPLEMENTED FOR STRUCT")
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    let loaded_config: ControllerConfig;
    let mut log_level: &str = "info";

    let arg_matches = App::new("srvctl")
        .about("Runs a controller that manages an Endpoint or EndpointSlice representing an SRV DNS record in Kubernetes")
        .arg(
            Arg::new("config")
                .long("config-file")
                .short('c')
                .value_name("FILE")
                .about("A config file used by the controller in either TOML or YAML format")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("verbosity")
                .long("log-level")
                .short('v')
                .about("Log level. One of trace, debug, info, warn, error. Can be overwritten via RUST_LOG env var")
                .default_value("info")
                .takes_value(true)
        )
        .get_matches();

    // configure logging appropriately
    if arg_matches.is_present("verbosity") {
        log_level = arg_matches.value_of("verbosity").unwrap();
    }

    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", &log_level);
    }

    std::env::set_var("RUST_LOG_STYLE", "never");
    env_logger::init();

    if log_level.eq("debug") || log_level.eq("trace") {
        debug!("Debug-level logging enabled")
    }

    let _ = resolve_srv("_mongodb._tcp.cv-eas-us-qa-eastus2-mo.nhtn2.azure.mongodb.net").await?;

    // first thing's first - we parse the config file supplied
    if arg_matches.is_present("config") {
        if let Some(conf) = arg_matches.value_of_os("config") {
            loaded_config = parse_load_config(&conf.to_str().unwrap()).unwrap();
        }
    }

    // loop {
    // info!("Beginning domain resolution for configured domains");
    // for dom in &loaded_config.domains {
    // let res = resolve_srv(dom.as_str()).await?;
    // }
    // }
    Ok(())
}

/// Handles the loading and parsing of the config file supplied as a CLI arg.
///
/// In order, it determines if the file path exists, extracts the extension,
/// and (for a supported file extension) attemptes to parse the file.
///
/// Returns either the error returned by the parser or a ControllerConfig struct.
fn parse_load_config(file_path: &str) -> anyhow::Result<ControllerConfig, anyhow::Error> {
    let fpath = Path::new(&file_path);
    if fpath.exists() {
        debug!(
            "Configuration file at path {} exists, loading file.",
            file_path
        );

        let f = File::open(fpath)?;
        let mut buf = BufReader::new(f);
        let mut file_contents = String::new();
        buf.read_to_string(&mut file_contents)?;

        debug!("Successfully loaded config file into string.");

        if file_contents.len() <= 0 {
            error!("Provided config file is empty, unable to continue parsing.");
            return Err(anyhow!(
                "The supplied configuration file is empty, unable to configure the controller"
            ));
        }

        match fpath.extension() {
            Some(ext) => match ext.to_str().unwrap() {
                "toml" => {
                    debug!("Config file has extension of 'toml', loading via TOML parser");
                    let loaded_config = toml::from_str(file_contents.as_str())?;
                    return Ok(loaded_config);
                }
                "yaml" => {
                    debug!("Config file has extension of 'yaml', loading via YAML parser");
                    let loaded_config = serde_yaml::from_str(file_contents.as_str())?;
                    return Ok(loaded_config);
                }
                _ => {
                    return Err(anyhow::Error::msg(format!(
                        "Unsupported configuration format '{:#?}'",
                        &ext
                    )))
                }
            },
            None => {
                return Err(anyhow::Error::msg(
                    "Error encountered while trying to discover config file format",
                ))
            }
        }
    } else {
        return Err(anyhow!(
            "The configuration file located at path '{}' does not exist!",
            file_path
        ));
    }
}
