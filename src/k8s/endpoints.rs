use k8s_openapi::{
    api::core::v1::Endpoints,
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
use super::gen_obj_meta;

use std::collections::BTreeMap;

pub async fn gen_endpoint_slices(
    client: &kube::Client,
    res: &dns::SrvResult,
    dom: SRVDomain,
    namespace: &str,
    labels: &BTreeMap<String, String>,
) -> anyhow::Result<()> {
    let mut srv_port: EndpointPort = EndpointPort {
        app_protocol: None,
        name: None,
        port: None,
        protocol: None,
    };
    let mut srv_endpoints: Vec<Endpoint> = vec![];

    if let Some(recs) = &res.srv_records {
        // KLUDGE we have to clone these (not ideal) to get around a missing Copy trait on
        // the SrvResult object
        let proto = res.protocol.clone();
        let name = res.service.clone();

        srv_port.protocol = proto;
        srv_port.name = name;
        srv_port.port = Some(recs.first().unwrap().port as i32);

        match dom.slice_type.as_str() {
            "ipv4" | "fqdn" => {
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

    // let labels = gen_common_labels();
    let meta = gen_obj_meta(dom.clone(), namespace, None, Some(labels.clone())).await?;

    let endpoint_obj = EndpointSlice {
        address_type: String::from(dom.slice_type),
        endpoints: srv_endpoints,
        ports: Some(vec![srv_port]),
        metadata: meta,
    };

    // time to actually create the endpointslice in kubernetes
    let params = PostParams::default();
    endpoint_api.create(&params, &endpoint_obj).await?;

    Ok(())
}

pub fn gen_endpoints(client: &kube::Client, res: &dns::SrvResult) {
    unimplemented!("Endpoint generation not yet implemented")
}
