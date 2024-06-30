use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

use log::{debug, info};
use rocket::{get, put, Responder, State};
use rocket::fs::TempFile;
use rocket::http::{Header, Status};
use rocket::response::status;
use rocket::response::status::Unauthorized;
use rocket::serde::json::Json;
use tempfile::NamedTempFile;

use crate::{ARProxyConfiguration, ManagedResourceAccess};
use crate::auth::ApiCredentials;
use crate::err::{BasicError, IOError, RepositoryNotFound};

#[get("/<repository>/<path..>", rank = 3)]
pub async fn get_repository_resource(
    repository: &str,
    path: PathBuf,
    resource_access: &ManagedResourceAccess,
    configuration: &State<ARProxyConfiguration>,
) -> Result<File, status::Custom<Json<BasicError>>> {
    let repository = configuration.repositories.get(repository).ok_or(
        BasicError::from(Box::new(RepositoryNotFound(repository.to_string())))
    )?;

    info!("Fetching resource: '{}' from repository: '{}'", path.to_str().unwrap(), repository);

    let resource_path = PathBuf::new()
        .join(repository)
        .join(path);

    debug!("Full resource path: '{}'", resource_path.to_str().unwrap());

    let arc = Arc::clone(&resource_access);
    let resource_path = arc.get_resource(
        resource_path
    ).await.map_err(|e| BasicError::from(e))?;

    let file = File::open(resource_path)
        .map_err(|e| BasicError::from(Box::new(IOError(e))))?;

    Ok(file)
}


#[put("/<repository>/<path..>", data = "<body_file>")]
pub async fn put_repository_resource<'a>(
    _name: &ApiCredentials,
    repository: &str,
    path: PathBuf,
    mut body_file: TempFile<'_>,
    resource_access: &ManagedResourceAccess,
    configuration: &State<ARProxyConfiguration>,
) -> Result<(), status::Custom<Json<BasicError>>> {
    let repository = configuration.repositories.get(repository).ok_or(
        BasicError::from(Box::new(RepositoryNotFound(repository.to_string())))
    )?;

    info!("Putting resource: '{}' to repository: '{}'", path.to_str().unwrap(), repository);

    let resource_path = PathBuf::new()
        .join(repository)
        .join(path);

    debug!("Full resource path: '{}'", resource_path.to_str().unwrap());

    let file = NamedTempFile::new()
        .map_err(|err| status::Custom(
            Status::InternalServerError,
            Json::<BasicError>(err.into()),
        ))?;

    body_file.persist_to(file.path())
        .await
        .map_err(|err| status::Custom(
            Status::InternalServerError,
            Json::<BasicError>(err.into()),
        ))?;

    Arc::clone(&resource_access).put_resource(
        resource_path,
        file.into_temp_path(),
    ).await.map_err(|e|
    BasicError::from(e)
    )
}

#[get("/")]
pub async fn home() -> &'static str {
    "Hello! This is the GCP Artifact Registry proxy written in Rust on Rocket."
}

#[derive(Responder)]
pub struct AuthRequestResponse {
    body: String,
    more: Header<'static>,
}

#[get("/authenticated", rank = 2)]
pub async fn un_authenticated() -> Unauthorized<AuthRequestResponse> {
    return Unauthorized(AuthRequestResponse {
        body: "Please authenticate".to_string(),
        more: Header::new("WWW-Authenticate", r#"Basic realm="""#),
    });
}

#[get("/authenticated", rank = 1)]
pub async fn authenticated(
    _api: &ApiCredentials
) -> &'static str {
    "Good work! You are authenticated."
}