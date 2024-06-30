use std::path::PathBuf;

use rocket::async_trait;
use tempfile::TempPath;

use crate::err::SerializableError;

#[async_trait]
pub trait ResourceAccess {
    async fn get_resource(
        & self,
        path: PathBuf
    ) -> Result<TempPath, Box<dyn SerializableError>>;

    async fn  put_resource(
        & self,
        path: PathBuf,
        file: TempPath,
    ) -> Result<(), Box<dyn SerializableError>>;
}

// pub struct Resource {
//     pub stream: ByteStream<Vec<u8>>,
// }