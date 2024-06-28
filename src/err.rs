use std::fmt::Debug;
use std::io::Error;
use rocket::http::Status;
use rocket::response::status;
use rocket::serde::json::Json;
use rocket::serde::Serialize;

pub trait SerializableError: Debug {
    fn name(&self) -> &'static str;

    fn message(&self) -> String;

    fn status(&self) -> u16;
}


#[derive(Serialize)]
pub struct BasicError {
    error_name: &'static str,
    message: String,
}

impl BasicError {
    pub fn from(err: Box<dyn SerializableError>) -> status::Custom<Json<BasicError>> {
        status::Custom(Status::from_code(err.status()).unwrap(), Json(BasicError {
            error_name: err.name(),
            message: err.message().clone(),
        }))
    }
}

impl Into<BasicError> for Error {
    fn into(self) -> BasicError {
        BasicError {
            error_name: "File system exception",
            message: self.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct RepositoryNotFound(pub String);

impl SerializableError for RepositoryNotFound {
    fn name(&self) -> &'static str {
        "Repository not found"
    }

    fn message(&self) -> String {
        format!("Failed to find repository: '{}'", self.0)
    }

    fn status(&self) -> u16 {
        404
    }
}