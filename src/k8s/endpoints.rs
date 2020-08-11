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

use super::super::dns;
use super::super::SRVDomain;

pub fn gen_endpoint_slices(
    client: &kube::Client,
    res: &dns::SrvResult,
    dom: &SRVDomain,
    namespace: &str,
) {
    let mut srv_port: EndpointPort;
    let mut srv_endpoints: Vec<Endpoint>;

    if let Some(recs) = &res.srv_records {
        // KLUDGE we have to clone these (not ideal) to get around a missing Copy trait on
        // the SrvResult object
        let proto = res.protocol.clone();
        let name = res.service.clone();

        let srv_port = EndpointPort {
            protocol: proto,
            name: name,
            port: Some(recs.first().unwrap().port as i32),
            app_protocol: None,
        };

        match dom.slice_type.as_str() {
            "ipv4" | "fqdn" => {
                srv_endpoints = vec![];

                recs.iter().for_each(|rec| {
                    let mut ips: Vec<String> = vec![];
                    if let Some(ipaddrs) = &rec.ipv4_addr {
                        for ip in ipaddrs {
                            ips.push(ip.to_string());
                        }
                    }
                    let e = k8s_openapi::api::discovery::v1beta1::Endpoint {
                        addresses: ips,
                        conditions: None,
                        hostname: Some(rec.hostname.clone()),
                        topology: None,
                        target_ref: None,
                    };
                    srv_endpoints.push(e);
                });
            }
            "ipv6" => {
                warn!("slice_type requested as IPv6 which is currently unsupported.");
                unimplemented!("This configuration (IPv6 support) is currently unimplemented.")
            }
            _ => {}
        }
    }

    let endpoint_api: Api<EndpointSlice> = Api::namespaced(client.clone(), namespace);
    let endpoint_obj = EndpointSlice {
        address_type: String::from(dom.slice_type),
        endpoints: srv_endpoints,
        ports: Some(vec![srv_port]),
        metadata: ObjectMeta {
            name: Some(dom.service_name),
            namespace: Some(String::from(namespace)),
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
}

pub fn gen_endpoints(client: &kube::Client, res: &dns::SrvResult) {}
