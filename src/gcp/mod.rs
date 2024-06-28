use crate::gcp::gcp_creds::{ArtifactRegistryCreds, GCPTokenError, retrieve_creds};

pub mod gcp_resource_access;
mod gcp_creds;

pub(crate) fn gcp_creds() -> Result<ArtifactRegistryCreds, GCPTokenError> {
    retrieve_creds()
}