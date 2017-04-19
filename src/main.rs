#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate pulldown_cmark;
#[macro_use] extern crate serde_derive;

use std::io::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};

use pulldown_cmark::{html, Parser};
use rocket_contrib::Template;
use rocket::response::NamedFile;

mod view;


#[derive(Serialize)]
struct TemplateContext {
    view_groups: Vec<view::ViewGroup>,
    content: String,
    title: String,
}



fn get_html(file: &str) -> String {
    let mut file = File::open(file).expect("Unable to open markdown file");
    let mut file_content = String::new();
    file.read_to_string(&mut file_content).expect("Unable to read file");
    
    let parser = Parser::new(&file_content);
    let mut bfr = String::new();
    html::push_html(&mut bfr, parser);
    bfr
}

#[get("/static/<file..>")]
fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}

#[get("/")]
fn get() -> Template {
    let file = r"C:\Dev\wiki\todo.md";
    let content = get_html(file);
    
    let groups = view::get_groups();
    let context = TemplateContext {
        title: "todo".into(),
        view_groups: groups,
        content: content,
    };

    let template = "test";
    println!("Opening template {}", template);
    Template::render(template, &context)
}

fn main() {
    rocket::ignite().mount("/", routes![get, files]).launch();
}