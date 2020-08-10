use anyhow::anyhow;
use clap::{App, Arg};
use k8s_openapi::{
    api::core::v1::{Endpoints, Service},
    api::discovery::v1beta1::{Endpoint, EndpointPort, EndpointSlice},
    apimachinery::pkg::apis::meta::v1::ObjectMeta,
};
use kube::{
    api::{DeleteParams, ListParams, Meta, PatchParams, PatchStrategy, PostParams},
    Api, Client,
};
use log::{debug, error, info, trace, warn};
use serde::Deserialize;
use serde_json::json;

use srvctl::dns::resolve_srv;
use srvctl::k8s::endpoints;
use srvctl::k8s::services;

use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
struct SRVDomain {
    hostname: String,
    #[serde(alias = "serviceName")]
    service_name: String,
    #[serde(alias = "sliceType")]
    slice_type: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ControllerConfig {
    domains: Vec<SRVDomain>,
    namespace: String,
    #[serde(alias = "clusterVersion")]
    cluster_version: String,
}

impl std::fmt::Display for ControllerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UNIMPLEMENTED FOR STRUCT")
    }
}

impl ControllerConfig {
    fn empty() -> Self {
        ControllerConfig {
            domains: vec![],
            namespace: String::new(),
            cluster_version: String::new(),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut loaded_config: ControllerConfig = ControllerConfig::empty();
    let mut log_level: &str = "info";
    let mut endpoint_slices_enabled = false; // setting a default here just in case

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
        .arg(
            Arg::with_name("endpoint-slice")
                .long("enable-endpoint-slices")
                .about("Sets a preference for EndpointSlices when creating services in-cluster. Boolean with a default of 'false'.")
                .about_long("Sets a preference for EndpointSlices instead of Endpoints when creating service representations in-cluster.

Accepts a boolean value and defaults to false. Note that EndpointSlices went into beta with 1.17, so your cluster may not have them enabled.")
                .default_value(false)
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

    let _ = resolve_srv("_mongodb._tcp.cv-eas-us-qa-eastus2-mo.nhtn2.azure.mongodb.net")
        .await
        .unwrap();

    // first thing's first - we parse the config file supplied
    if arg_matches.is_present("config") {
        if let Some(conf) = arg_matches.value_of_os("config") {
            loaded_config = parse_load_config(&conf.to_str().unwrap()).unwrap();
        }
    } else {
        error!(
            "No configuration file arg has been supplied - can't proceed so exiting with an error"
        );
    }

    endpoint_slices_enabled = arg_matches.value_of("endpoint-slices").unwrap().eq("true");

    let client = Client::try_default().await?;
    loop {
        info!("Beginning domain resolution for configured domains");

        for dom in &loaded_config.domains {
            debug!("Generating SrvResult vector for hostname {:#?}", dom);

            let res = resolve_srv(dom.hostname.as_str()).await?;
            let mut srv_endpoints: Vec<Endpoint>;
            let mut srv_port: EndpointPort;

            if let Some(recs) = res.srv_records {
                // generate a BTreeMap with initial labels
                // TODO: Move this into a more configuration-friendly method.
                let mut labels: BTreeMap<String, String> = BTreeMap::new();
                labels.insert(
                    String::from("app.kubernetes.io/service-name"),
                    dom.clone().service_name,
                );
                labels.insert(String::from("srvctl.tsp.tc/srv-hostname"), res.srv_hostname);
                labels.insert(
                    String::from("app.kubernetes.io/managed-by"),
                    String::from("srvctl"),
                );

                if endpoint_slices_enabled {
                    endpoints::gen_endpoint_slices(
                        &client,
                        &res,
                        dom.slice_type.as_str(),
                        &loaded_config.namespace.as_str(),
                    );
                } else {
                    endpoints::gen_endpoints(&client, &res);
                }
            } else {
                warn!("No resolved records returned for {}, nothing to create in-cluster representation of.", dom.hostname);
                debug!("SrvResult from resolve_srv: {:#?}", res);
                break;
            }

            let service: Api<Service> =
                Api::namespaced(client.clone(), &loaded_config.clone().namespace);

            if loaded_config.cluster_version.eq("1.17") {
                endpoint_api: Api<EndpointSlice> =
                    Api::namespaced(client.clone(), &loaded_config.clone().namespace);
            } else {
                endpoint_api: Api<Endpoints> =
                    Api::namespaced(client.clone(), &loaded_config.clone().namespace);
            }

            let endpoint: Api<Endpoints> =
                Api::namespaced(client.clone(), &loaded_config.clone().namespace);

            // TODO we should only need to create this for clusters that are >= 1.17
            let endpoint_slice: Api<EndpointSlice> =
                Api::namespaced(client.clone(), &loaded_config.clone().namespace);

            let slice_obj = EndpointSlice {
                address_type: dom.slice_type.clone(),
                endpoints: srv_endpoints,
                ports: Some(vec![srv_port]),
                metadata: ObjectMeta {
                    name: Some(dom.service_name.clone()),
                    namespace: Some(loaded_config.clone().namespace),
                    annotations: None,
                    cluster_name: None,
                    creation_timestamp: None,
                    deletion_grace_period_seconds: None,
                    deletion_timestamp: None,
                    finalizers: None,
                    generate_name: None,
                    generation: None,
                    managed_fields: None,
                    owner_references: None, // TODO: generate correct owner data
                    resource_version: None,
                    self_link: None,
                    uid: None,
                    labels: Some(labels),
                },
            };

            // create the endpoint or endpointslice
            // TODO:: Clean this up with support for endpoints _or_ endpointslices
        }
    }
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
