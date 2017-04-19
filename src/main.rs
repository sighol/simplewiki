#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate pulldown_cmark;
extern crate regex;
#[macro_use] extern crate serde_derive;

use std::path::{Path, PathBuf};

use rocket_contrib::Template;
use rocket::response::NamedFile;

mod view;

#[derive(Serialize)]
struct TemplateContext {
    view_groups: Vec<view::ViewGroup>,
    content: String,
    title: String,
    prev_url: String,
    next_url: String,
    page: String,
}

fn get_html(file: &Path) -> String {
    use std::io::prelude::*;
    use std::fs::File;
    use pulldown_cmark::{html, Parser};

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

#[get("/<category>/<page>")]
fn get(category: &str, page: &str) -> Template {
    let wiki_root = Path::new(r"C:\Dev\wiki");
    
    let file_name = format!("{}.md", page);
    let page_name = format!("{}/{}", category, page);

    let path = wiki_root.join(category).join(file_name);

    let content = get_html(&path);
    
    let view_finder = view::ViewFinder::new(wiki_root.to_owned());
    let groups = view_finder.get_groups().expect("Unable to read wiki directory");
    let prev_next = view::find_prev_next(&groups, &page_name);
    let context = TemplateContext {
        prev_url: prev_next.prev.map_or("".into(), |p| p.file_name),
        next_url: prev_next.next.map_or("".into(), |p| p.file_name),
        title: page.into(),
        page: page_name,
        view_groups: groups,
        content: content,
    };

    let template = "test";
    println!("Opening template {}", template);
    Template::render(template, &context)
}

fn main() {
    rocket::ignite().mount("/", routes![files, get]).launch();
}