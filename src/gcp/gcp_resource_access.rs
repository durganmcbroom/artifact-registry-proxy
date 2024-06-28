use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bytes::Bytes;
use reqwest::{Client, StatusCode};
use reqwest::header::HeaderValue;
use rocket::async_trait;

use ArtifactRegistryResourceFetchError::{NonSuccessfulStatus, RequestError, TokenError};

use crate::err::SerializableError;
use crate::gcp::gcp_creds::{ArtifactRegistryCreds, GCPTokenError};
use crate::gcp::gcp_resource_access::ArtifactRegistryResourceFetchError::IOError;
use crate::resource_access::ResourceAccess;

pub struct ArtifactRegistryResourceAccess {
    pub creds: ArtifactRegistryCreds,
    pub url: String,
}

#[derive(Debug)]
pub enum ArtifactRegistryResourceFetchError {
    RequestError(reqwest::Error),
    NonSuccessfulStatus(StatusCode),
    TokenError(GCPTokenError),
    IOError(std::io::Error),
}

impl SerializableError for ArtifactRegistryResourceFetchError {
    fn name(&self) -> &'static str {
        match self {
            RequestError(_) => { "Exceptional request exception" }
            NonSuccessfulStatus(_) => { "Non-200 internal response" }
            TokenError(_) => { "Token error" }
            IOError(_) => { "File IO Exception" }
        }
    }

    fn message(&self) -> String {
        match self {
            RequestError(err) => {
                format!(
                    "Failed to request resource, wrapped error: {}",
                    err.to_string()
                )
            }
            NonSuccessfulStatus(status) => {
                format!("Recieved response code: '{}' from the Artifact Registry servers", status.as_str())
            }
            TokenError(it) => {
                format!("Failed to authenticate with teh Artifact Registry servers! {}", it)
            }
            IOError(e) => {
                format!("An IO exception has occurred internally: '{}'", e.to_string())
            }
        }
    }

    fn status(&self) -> u16 {
        match self {
            RequestError(_) => { 400 }
            NonSuccessfulStatus(status) => { status.as_u16() }
            TokenError(_) => { 400 }
            IOError(_) => { 400 }
        }
    }
}

impl ArtifactRegistryResourceAccess {
    fn get_url(&self, path: PathBuf) -> String {
        let cloned_url = self.url.clone();
        format!(
            "{}/{}",
            match cloned_url.strip_suffix("/") {
                Some(t) => t.to_string(),
                None => cloned_url
            },
            path.to_str().unwrap()
        )
    }

    async fn encoded_creds(&self) -> Result<String, Box<dyn SerializableError>> {
        let key = self.creds.get_key()
            .await
            .map_err(|err| Box::new(TokenError(err)) as Box<dyn SerializableError>)?.clone();

        let encoded_creds = BASE64_STANDARD
            .encode(format!("{}:{}", self.creds.user, key));

        Ok(encoded_creds)
    }
}

#[async_trait]
impl ResourceAccess for ArtifactRegistryResourceAccess {
    async fn get_resource(&self, path: PathBuf) -> Result<Bytes, Box<dyn SerializableError>> {
        let response = Client::new()
            .get(self.get_url(path))
            .header(
                "Authorization",
                HeaderValue::from_str(
                    format!(
                        "Basic {}",
                        self.encoded_creds().await?
                    ).as_str()
                ).unwrap(),
            )
            .send()
            .await
            .map_err(|err| Box::new(RequestError(err)) as Box<dyn SerializableError>)?;

        if !response.status().is_success() {
            return Err(Box::new(NonSuccessfulStatus(response.status())) as Box<dyn SerializableError>);
        }

        let stream = response.bytes()
            .await
            .map_err(|err| Box::new(RequestError(err)) as Box<dyn SerializableError>)?;

        Ok(stream)
    }

    async fn put_resource(
        &self,
        path: PathBuf,
        file: &mut File,
        // stream: impl Stream<Item=Result<Box<[u8]>, Box<dyn Error + Send + Sync>>> + Send + Sync + 'static,
    ) -> Result<(), Box<dyn SerializableError>> {
        let mut body = Vec::new();
        file.read_to_end(&mut body).map_err(|e| Box::new(IOError(e)) as Box<dyn SerializableError>)?;

        let content_length = body.len();
        let response = Client::new()
            .put(self.get_url(path))
            .body(body)
            .header(
                "Authorization",
                HeaderValue::from_str(
                    format!(
                        "Basic {}",
                        self.encoded_creds().await?
                    ).as_str()
                ).unwrap(),
            )
            .header(
                "Content-Length",
                content_length,
            )
            .send()
            .await
            .map_err(|err| Box::new(RequestError(err)) as Box<dyn SerializableError>)?;

        if !response.status().is_success() {
            return Err(Box::new(NonSuccessfulStatus(response.status())) as Box<dyn SerializableError>);
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::err::SerializableError;
    use crate::gcp::gcp_creds::retrieve_creds;
    use crate::gcp::gcp_resource_access::ArtifactRegistryResourceAccess;
    use crate::gcp::gcp_resource_access::ArtifactRegistryResourceFetchError::TokenError;
    use crate::resource_access::ResourceAccess;

    #[tokio::test]
    async fn test_resource_get() -> Result<(), Box<dyn SerializableError>> {
        let mut access = ArtifactRegistryResourceAccess {
            creds: retrieve_creds().map_err(|err| {
                Box::new(TokenError(err)) as Box<dyn SerializableError>
            })?,
            url: "https://us-central1-maven.pkg.dev/extframework/maven-snapshots".to_string(),
        };

        let resource = access.get_resource(
            PathBuf::from("com/durganmcbroom/jobs-jvm/1.2-SNAPSHOT/maven-metadata.xml"),
        ).await?;

        println!("{}", String::from_utf8(resource.to_vec()).unwrap());

        Ok(())
    }
}