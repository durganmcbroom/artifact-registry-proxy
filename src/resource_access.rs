use std::fs::File;
use std::path::PathBuf;

use rocket::async_trait;

use crate::err::SerializableError;

#[async_trait]
pub trait ResourceAccess {
    async fn get_resource(
        & self,
        path: PathBuf
    ) -> Result<bytes::Bytes, Box<dyn SerializableError>>;

    async fn put_resource(
        & self,
        path: PathBuf,
        file: &mut File,
    ) -> Result<(), Box<dyn SerializableError>>;
}

// pub struct Resource {
//     pub stream: ByteStream<Vec<u8>>,
// }