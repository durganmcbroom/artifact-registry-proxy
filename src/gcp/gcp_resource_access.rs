use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use log::info;
use reqwest::{Client, StatusCode};
use reqwest::header::HeaderValue;
use rocket::async_trait;
use tempfile::{Builder, TempPath};

use ArtifactRegistryResourceFetchError::{NonSuccessfulStatus, RequestError, TokenError};

use crate::err::{IOError, SerializableError};
use crate::gcp::gcp_creds::{ArtifactRegistryCreds, GCPTokenError};
use crate::gcp::gcp_resource_access::ArtifactRegistryResourceFetchError::InvalidPathBuf;
use crate::resource_access::ResourceAccess;

pub struct ArtifactRegistryResourceAccess {
    pub creds: ArtifactRegistryCreds,
    pub url: String,
    // pub cache_path: Path,
}

#[derive(Debug)]
pub enum ArtifactRegistryResourceFetchError {
    RequestError(reqwest::Error),
    NonSuccessfulStatus(StatusCode, String),
    TokenError(GCPTokenError),
    InvalidPathBuf,
}

impl SerializableError for ArtifactRegistryResourceFetchError {
    fn name(&self) -> &'static str {
        match self {
            RequestError(_) => { "Exceptional request exception" }
            NonSuccessfulStatus(_, _) => { "Non-200 internal response" }
            TokenError(_) => { "Token error" }
            InvalidPathBuf => { "Invalid path supplied" }
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
            NonSuccessfulStatus(status, body) => {
                format!("Received response code: '{}' from the Artifact Registry servers. Body: {}", status.as_str(), body)
            }
            TokenError(it) => {
                format!("Failed to authenticate with teh Artifact Registry servers! {}", it)
            }
            InvalidPathBuf => { "Given path did no have a valid file ending (eg 'test.txt')".to_string() }
        }
    }

    fn status(&self) -> u16 {
        match self {
            RequestError(_) => { 500 }
            NonSuccessfulStatus(status, _) => { status.as_u16() }
            TokenError(_) => { 500 }
            InvalidPathBuf => { 400 }
        }
    }
}

impl ArtifactRegistryResourceAccess {
    fn get_url(&self, path: &PathBuf) -> String {
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
    async fn get_resource(&self, path: PathBuf) -> Result<TempPath, Box<dyn SerializableError>> {
        let url = self.get_url(&path);
        info!("Request resource from: '{}'", url);

        let response = Client::new()
            .get(url)
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
            return Err(Box::new(NonSuccessfulStatus(response.status(), response.text().await.unwrap_or("<Failed to unwrap body data>".to_string()))) as Box<dyn SerializableError>);
        }

        let stream = response.bytes()
            .await
            .map_err(|err| Box::new(RequestError(err)) as Box<dyn SerializableError>)?;

        let mut file = Builder::new()
            .prefix(path.file_stem().ok_or_else(|| Box::new(InvalidPathBuf) as Box<dyn SerializableError>)?)
            .suffix(format!(".{}", path.extension().ok_or_else(|| Box::new(InvalidPathBuf) as Box<dyn SerializableError>)?.to_str()
                .ok_or_else(|| Box::new(InvalidPathBuf) as Box<dyn SerializableError>)?
            ).as_str())
            .tempfile().map_err(|e| Box::new(IOError(e)) as Box<dyn SerializableError>)?;

        for chunk in stream.chunks(64) {
            file.write_all(chunk).map_err(|e| Box::new(IOError(e)) as Box<dyn SerializableError>)?
        }

        Ok(file.into_temp_path())
    }

    async fn put_resource(
        &self,
        path: PathBuf,
        file: TempPath,
    ) -> Result<(), Box<dyn SerializableError>> {
        let mut body = Vec::new();

        File::open(file)
            .map_err(|e| Box::new(IOError(e)) as Box<dyn SerializableError>)?
            .read_to_end(&mut body).map_err(|e| Box::new(IOError(e)) as Box<dyn SerializableError>)?;

        let content_length = body.len();

        let url = self.get_url(&path);
        info!("Put resource to: '{}'", url);

        let response = Client::new()
            .put(url)
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
            return Err(Box::new(NonSuccessfulStatus(
                response.status(),
                response.text().await.unwrap_or("<Failed to unwrap body data>".to_string()))) as Box<dyn SerializableError>
            );
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{Read, Write};
    use std::path::PathBuf;

    use tempfile::Builder;

    use crate::err::SerializableError;
    use crate::gcp::gcp_creds::retrieve_creds;
    use crate::gcp::gcp_resource_access::ArtifactRegistryResourceAccess;
    use crate::gcp::gcp_resource_access::ArtifactRegistryResourceFetchError::TokenError;
    use crate::resource_access::ResourceAccess;
    use crate::setup_logging;

    #[tokio::test]
    async fn test_resource_get() -> Result<(), Box<dyn SerializableError>> {
        let access = ArtifactRegistryResourceAccess {
            creds: retrieve_creds().map_err(|err| {
                Box::new(TokenError(err)) as Box<dyn SerializableError>
            })?,
            url: "https://us-central1-maven.pkg.dev/extframework/maven-snapshots".to_string(),
        };

        let resource = access.get_resource(
            PathBuf::from("a/b/a/test.txt"),
        ).await?;

        let mut buf = Vec::new();
        File::open(resource).unwrap().read_to_end(&mut buf).unwrap();
        println!("{}", String::from_utf8(buf).unwrap());

        Ok(())
    }

    #[tokio::test]
    async fn test_resource_put() -> Result<(), Box<dyn SerializableError>> {
        setup_logging().unwrap();
        let access = ArtifactRegistryResourceAccess {
            creds: retrieve_creds().map_err(|err| {
                Box::new(TokenError(err)) as Box<dyn SerializableError>
            })?,
            url: "https://us-central1-maven.pkg.dev/extframework/maven-snapshots".to_string(),
        };

        let buf = PathBuf::from("a/b/a/test.txt");

        let mut file = Builder::new()
            .prefix(buf.file_stem().unwrap())
            .tempfile().unwrap();

        write!(file, "Hey i did this!").unwrap();

        access.put_resource(
            PathBuf::from("a/b/a/test.txt"),
            file.into_temp_path(),
        ).await?;

        Ok(())
    }
}