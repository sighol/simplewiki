extern crate includedir_codegen;

use includedir_codegen::Compression;

fn main() {
    includedir_codegen::start("STATIC_FILES")
        .dir("static", Compression::None)
        .build("generated_static.rs")
        .unwrap();

    includedir_codegen::start("TEMPLATE_FILES")
        .dir("templates", Compression::None)
        .build("generated_template.rs")
        .unwrap();
}
