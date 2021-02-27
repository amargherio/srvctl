use k8s_openapi::{
    api::core::v1::ConfigMap,
    apimachinery::pkg::apis::meta::v1::{ObjectMeta, OwnerReference},
};
use kube::{
    api::{ListParams, Meta, PatchParams, PatchStrategy},
    Api, Client,
};
use kube_derive::CustomResource;
use kube_runtime::controller::{Context, Controller, ReconcilerAction};
use serde::{Deserialize, Serialize};

// SRVResolver CRD
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "srv.amargher.io", version = "v1", kind = "SRVResolver")]
#[kube(apiexteions = "v1beta1")]
#[kube(status = "SRVResolverStatus")]
pub struct SRVResolver {
    srv_record: &str,
    include_txt: bool,
}