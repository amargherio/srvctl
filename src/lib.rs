pub mod dns;
pub mod k8s;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct SRVDomain {
    pub hostname: String,
    #[serde(alias = "serviceName")]
    pub service_name: String,
    #[serde(alias = "sliceType")]
    pub slice_type: String,
}
