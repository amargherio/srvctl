pub mod endpoints;
pub mod services;

use std::collections::BTreeMap;

pub(crate) fn gen_common_labels() -> BTreeMap<String, String> {
    let labels = BTreeMap::new();

    labels.insert(
        String::from("app.kubernetes.io/service-name"),
        dom.clone().service_name,
    );
    labels.insert(String::from("srvctl.tsp.tc/srv-hostname"), srv_hostname);
    labels.insert(
        String::from("app.kubernetes.io/managed-by"),
        String::from("srvctl"),
    );
}
