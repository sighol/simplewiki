use std::path::{Path, PathBuf};
use std::io::{self};

use rocket::request::Request;
use rocket::response::{Response, Responder};
use rocket::http::{Status, ContentType};

include!(concat!(env!("OUT_DIR"), "/generated_static.rs"));

pub struct StaticFile {
    path: PathBuf
}

impl StaticFile {
    pub fn new(path: PathBuf) -> Self {
        let full_path = Path::new("static").join(path);
        StaticFile { path: full_path }
    }
}

impl Responder<'static> for StaticFile {
    fn respond_to(self, _: &Request) -> Result<Response<'static>, Status> {
        let mut response = Response::new();
        if let Some(ext) = self.path.extension() {
            if let Some(ct) = ContentType::from_extension(&ext.to_string_lossy()) {
                response.set_header(ct);
            }
        }

        let path = self.path.to_str().unwrap();

        match STATIC_FILES.read(path) {
            Ok(reader) => response.set_streamed_body(reader),
            Err(error) => return Err(Status::new(404, "Could not find static file")),
        }

        Ok(response)
    }
}
