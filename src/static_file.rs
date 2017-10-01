use std::path::{Path, PathBuf};
use std::io::{Write};
use std::env;
use std::fs::{self, File};

use rocket::request::Request;
use rocket::response::{Response, Responder};
use rocket::http::{Status, ContentType};

include!(concat!(env!("OUT_DIR"), "/generated_static.rs"));
include!(concat!(env!("OUT_DIR"), "/generated_template.rs"));

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
            Err(_) => return Err(Status::new(404, "Could not find static file")),
        }

        Ok(response)
    }
}

/// Extracts the template files and returns the full path to the extracted
/// folder
pub fn extract_templates() -> PathBuf {
    let tmp_dir: PathBuf = env::temp_dir().join("simplewiki");
    fs::create_dir_all(&tmp_dir).unwrap();
    for file_path in TEMPLATE_FILES.file_names() {

        let path = Path::new(file_path);
        let path = path.strip_prefix("templates").unwrap();
        let target_path = tmp_dir.join(path.to_str().unwrap());
        println!("File path: {}", file_path);

        let content = TEMPLATE_FILES.get(file_path).unwrap().into_owned();

        let mut file = File::create(&target_path).unwrap();
        file.write_all(&content).unwrap();

    }

    tmp_dir
}
