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

pub fn gen_endpoint_slices(
    client: &kube::Client,
    res: &dns::SrvResult,
    slice_type: &str,
    namespace: &str,
) {
    let mut srv_port: EndpointPort;
    let mut srv_endpoints: Vec<Endpoint>;

    if let Some(recs) = &res.srv_records {
        let srv_port = EndpointPort {
            protocol: res.protocol,
            name: res.service,
            port: Some(recs.first().unwrap().port as i32),
            app_protocol: None,
        };

        match slice_type {
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
                unimplemented!("This configuration (IPv6 support) is currently unimplemented.")
            }
            _ => {}
        }
    }

    let endpoint_api: Api<EndpointSlice> = Api::namespaced(client.clone(), namespace);
}

pub fn gen_endpoints(client: &kube::Client, res: &dns::SrvResult) {}
