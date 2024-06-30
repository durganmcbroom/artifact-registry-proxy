extern crate core;

use std::collections::HashMap;
use std::env;
use std::env::VarError;
use std::sync::Arc;
use dotenv::dotenv;

use rocket::{launch, routes, State};

use crate::auth::ApiCredentials;
use crate::gcp::gcp_creds;
use crate::gcp::gcp_resource_access::{ArtifactRegistryResourceAccess, ArtifactRegistryResourceFetchError};
use crate::resource_access::ResourceAccess;
use crate::routes::{authenticated, get_repository_resource, home, put_repository_resource, un_authenticated};

mod resource_access;
mod gcp;
pub mod err;
mod routes;
pub mod auth;

pub type ManagedResourceAccess = State<Arc<dyn ResourceAccess + Send + Sync>>;


struct ARProxyConfiguration {
    repositories: HashMap<String, String>,
    url: String,
    creds: ApiCredentials,
}

fn setup_configuration<'a>() -> ARProxyConfiguration {
    let url = env::var("GAR_API_URL").expect(
        "Cannot find the Google Artifact registry API URL (specified by the environmental variable: 'GAR_API_URL')"
    ).to_string();

    let repository_string = env::var("REPOSITORIES").expect(
        "Cannot find repository configuration in the environmental variables (formatted: 'public_name:gar_id,...') (specified by environmental variable: 'REPOSITORIES')"
    ).to_string();

    let mut repositories = HashMap::new();

    repository_string.split(",")
        .filter(|str| !str.is_empty())
        .map(|str| str.split(":"))
        .map(|split| split.collect::<Vec<&str>>())
        .for_each(|str| {
            repositories.insert(
                str.get(0).expect("Key expected for repositories!").to_string(),
                str.get(1).expect("Value expected for repositories!").to_string(),
            );
        });

    let binding = env::var("CREDENTIALS")
        .or_else(|_| Ok::<String, VarError>(":".to_string()));
    let (api_user, api_key) =
        binding
            .as_ref()
            .map(|t| t.split_once(":").expect("Invalid CREDENTIALS env specified, should be a string seperated by ':' (user:key).")).unwrap();

    let creds = ApiCredentials {
        user: api_user.to_string(),
        key: api_key.to_string(),
    };

    ARProxyConfiguration {
        repositories,
        url,
        creds,
    }
}

pub(crate) fn setup_logging() -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] [{}] {}",
                chrono::Local::now().format("[%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        .apply()?;
    Ok(())
}

#[launch]
fn launch() -> _ {
    #[cfg(debug_assertions)]
    {
        dotenv().unwrap();
    }
    setup_logging().expect("Failed to init fern logging.");

    let configuration = setup_configuration();

    rocket::build()
        .manage(Arc::new(
            ArtifactRegistryResourceAccess {
                creds: gcp_creds().map_err(|err| {
                    ArtifactRegistryResourceFetchError::TokenError(err)
                }).unwrap(),
                url: configuration.url.clone(),
            }
        ) as Arc<dyn ResourceAccess + Send + Sync>)
        .manage(configuration)
        .mount("/", routes![
            get_repository_resource,
            put_repository_resource,
            home,
            un_authenticated,
            authenticated
        ])
}