pub mod endpoints;
pub mod services;

use std::collections::BTreeMap;

use crate::SRVDomain;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;

pub(crate) fn gen_common_labels(
    service_name: &str,
    srv_hostname: &str,
) -> BTreeMap<String, String> {
    let labels = BTreeMap::new();

    labels.insert(
        String::from("app.kubernetes.io/service-name"),
        String::from(service_name),
    );
    labels.insert(
        String::from("srvctl.tsp.tc/srv-hostname"),
        String::from(srv_hostname),
    );
    labels.insert(
        String::from("app.kubernetes.io/managed-by"),
        String::from("srvctl"),
    );

    labels
}

pub(crate) async fn gen_obj_meta(
    domain: SRVDomain,
    namespace: &str,
    annotations: Option<BTreeMap<String, String>>,
    labels: Option<BTreeMap<String, String>>,
) -> anyhow::Result<ObjectMeta> {
    Ok(ObjectMeta {
        name: Some(domain.service_name),
        namespace: Some(String::from(namespace)),
        annotations: annotations,
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
        labels: labels,
    })
}
