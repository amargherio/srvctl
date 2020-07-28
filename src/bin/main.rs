use anyhow::Result;
use clap::{App, Arg, ArgMatches, Clap};
use log::{debug, error, info, trace, warn};

use crate::dns::resolve_srv;

#[derive(Debug, Deserialize, Display)]
struct ControllerConfig {
    domains: Vec<String>,
    #[serde(alias = "createService")]
    create_service: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let loaded_config: &ControllerConfig;
    let log_level: &str;

    let arg_matches = App::new("srvctl")
        .about("Runs a controller that manages an Endpoint or EndpointSlice representing an SRV DNS record in Kubernetes")
        .arg(
            Arg::new("config")
                .long("config")
                .value_name("FILE")
                .about("A config file used by the controller in either TOML or YAML format")
                .takes_value(true)
        )
        .arg(
            Arg::new("loglevel")
                .short("v")
                .log("loglevel")
                .multiple(true)
                .about("Log level. One of trace, debug, info, warn, error, or up to 3 consecutive flags '-vv'. Can be overwritten via RUST_LOG env var")
                .takes_value(true)
        )
        .get_matches();

    // configure logging appropriately
    if arg_matches.is_present("loglevel") {
        match matches.occurences_of("loglevel") {
            1 => {
                // we only received one. try to get a value and if there isn't
                // one, it goes to debug logging
                if let Some(lvl) = arg_matches.value_of("loglevel") {
                    log_level = lvl;
                } else {
                    log_level = "debug";
                }
            }
            2 => {
                log_level = "trace";
            }
        }
    }
    std::env::set_var("RUST_LOG", &log_level);
    std::env::set_var("RUST_LOG_STYLE", "never");
    env_logger::init();

    if log_level.eq("debug") || log_level.eq("trace") {
        debug!("Debug-level logging enabled")
    }

    // first thing's first - we parse the config file supplied
    if arg_matches.is_present("config") {
        if let Some(conf) = arg_matches.value_of_os("config") {
            loaded_config = parse_load_config(&conf)?;
        } else {
            Err(anyhow::Error::msg(
                "No configuration file provided, no domains to monitor",
            ))
        }
    }

    loop {
        info!("Beginning domain resolution for configured domains");
        for dom in &loaded_config.domains {
            let res = resolve_srv(dom.as_str)?;
        }
    }
}

/// Handles the loading and parsing of the config file supplied as a CLI arg.
///
/// In order, it determines if the file path exists, extracts the extension,
/// and (for a supported file extension) attemptes to parse the file.
///
/// Returns either the error returned by the parser or a ControllerConfig struct.
fn parse_load_config(file_path: &str) -> anyhow::Result<ControllerConfig> {
    let fpath = Path::new(&file_path);
    if fpath.exists() {
        debug!(
            "Configuration file at path {} exists, loading file.",
            file_path
        );

        let f = File::open(fpath);
        let buf = BufReader::new(f);
        let file_contents = String::new();
        buf.read_to_string(&mut file_contents)?;

        debug!("Successfully loaded config file into string.");

        if len(file_contents) == 0 {
            error!("Provided config file is empty, unable to continue parsing.");
            Err(anyhow::Error::msg(
                "The supplied configuration file is empty",
            ))
        }

        match fpath.extension() {
            "toml" => {
                debug!("Config file has extension of 'toml', loading via TOML parser");
                loaded_config = toml::from_str(file_contents);
                Ok(loaded_config)
            }
            "yaml" => {
                debug!("Config file has extension of 'yaml', loading via YAML parser");
                loaded_config = serde_yaml::from_str(&file_contents)?;
                Ok(loaded_config)
            }
            Some(ext) => Err(anyhow::Error::msg(format!(
                "Unsupported configuration format '{}'",
                &ext
            ))),
            _ => Err(anyhow::Error::msg(
                "Error encountered while trying to discover config file format",
            )),
        }
    } else {
        error!("Supplied configuration file does not exist, cannot start the controller.");
        Err(anyhow::Error::msg(format!(
            "Config file does not exist at {}",
            &file_path
        )))
    }
}
