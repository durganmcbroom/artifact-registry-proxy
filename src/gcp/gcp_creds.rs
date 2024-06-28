use std::fmt;
use std::fmt::{Display, Formatter};
use std::process::Command;

use chrono::{DateTime, FixedOffset};
use log::{debug, info};
use serde_json::Value;
use tokio::sync::RwLock;

use GCPTokenError::MalformedJsonCreds;

use crate::gcp::gcp_creds::GCPTokenError::ISOParse;

pub struct ArtifactRegistryCreds {
    pub user: &'static str,
    inner: RwLock<Inner>,
}

struct Inner {
    key: String,
    expiration: DateTime<FixedOffset>,
}

impl Display for ArtifactRegistryCreds {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ArtifactRegistryCreds(\n   user={},\n   key=<***>,\n   expiration=<***>\n)", self.user)
    }
}

#[derive(fmt::Debug)]
pub enum GCPTokenError {
    GCloudCommand(i32),
    SerdeError(serde_json::Error),
    MalformedJsonCreds(&'static str),
    ISOParse(),
}

impl Display for GCPTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let str = match self {
            GCPTokenError::GCloudCommand(status) => { format!("The GCloud command exited with error code: '{}'", status) }
            GCPTokenError::SerdeError(err) => { format!("Failed to parse JSON becuase: '{}'", err) }
            MalformedJsonCreds(err) => { err.to_string() }
            ISOParse() => { "Failed to parse an ISO Date!".to_string() }
        };
        write!(f, "{}", str)
    }
}

impl ArtifactRegistryCreds {
    pub async fn get_key(&self) -> Result<String, GCPTokenError> {
        let current_datetime = chrono::Local::now();

        let inner = self.inner.read().await;

        let offset = inner.expiration.signed_duration_since(current_datetime);

        if offset.num_minutes() <= 5 {
            drop(inner);
            info!("GCP Credentials are about to expire (within 5 minutes), refreshing now.");

            let (key, expiration) = retrieve_creds_internal()?;

            let mut inner = self.inner.write().await;

            inner.key = key;
            inner.expiration = expiration;
        }

        Ok(
            self.inner.read().await.key.clone()
        )
    }
}

pub fn retrieve_creds() -> Result<ArtifactRegistryCreds, GCPTokenError> {
    let (key, expiration) = retrieve_creds_internal()?;

    Ok(
        ArtifactRegistryCreds {
            user: "oauth2accesstoken",
            inner: RwLock::new(Inner {
                key,
                expiration,
            }),
        }
    )
}

fn retrieve_creds_internal() -> Result<(String, DateTime<FixedOffset>), GCPTokenError> {
    info!("Retrieving GCP credentials");

    let output = Command::new("gcloud")
        .args(["config", "config-helper", "--format=json(credential)"])
        .output()
        .expect("Failed to run the GCloud command!");

    match output.status.code() {
        None => { return Err(GCPTokenError::GCloudCommand(-1)) }
        Some(code) => {
            if code != 0 {
                return Err(GCPTokenError::GCloudCommand(code))
            }
        }
    }

    let value: Value = serde_json::from_slice(
        output.stdout.as_slice()
    ).map_err(|err| GCPTokenError::SerdeError(err))?;

    let creds_value = value.as_object().ok_or(MalformedJsonCreds("Expected object."))
        ?.get("credential").ok_or(MalformedJsonCreds("Failed to find 'credential' in json object."))
        ?.as_object().ok_or(MalformedJsonCreds("Expected object."))?;

    let access_token = creds_value
        .get("access_token").ok_or(MalformedJsonCreds("Expected object containing property: 'access_token'."))
        ?.as_str().ok_or(MalformedJsonCreds("Expected string in property: 'access_token', instead found something else"))?;

    let expiration = creds_value.get("token_expiry")
        .ok_or(MalformedJsonCreds("Failed to find property: 'token_expiry'"))?.as_str()
        .ok_or(MalformedJsonCreds("Property token_expiry should be a string"))?;

    info!("Retrieved new access token from GCP.");
    debug!("Token: '{}' expires {}", access_token, expiration);

    Ok((
        access_token.to_string(),
        chrono::DateTime::parse_from_rfc3339(expiration)
            .map_err(|_| ISOParse())?,
    ))
}

#[cfg(test)]
mod tests {
    use dotenv::dotenv;

    use crate::gcp::gcp_creds::{GCPTokenError, retrieve_creds};
    use crate::gcp::gcp_creds::GCPTokenError::ISOParse;

    #[test]
    fn test_iso_parsing() -> Result<(), GCPTokenError> {
        let expiration = "2024-06-20T05:00:17Z";

        let time = chrono::DateTime::parse_from_rfc3339(expiration)
            .map_err(|err| ISOParse())?;

        println!("{}", time);

        Ok(())
    }

    #[test]
    fn test_artifact_registry_auth() -> Result<(), GCPTokenError> {
        dotenv().unwrap();

        println!("{}", retrieve_creds()?);

        Ok(())
    }
}