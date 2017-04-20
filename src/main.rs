#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate pulldown_cmark;
extern crate regex;
#[macro_use] extern crate serde_derive;

use std::io;
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

fn get_html(file: &Path) -> io::Result<String> {
    use std::io::prelude::*;
    use std::fs::File;
    use pulldown_cmark::{html, Parser};

    let mut file = File::open(file)?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;
    
    let parser = Parser::new(&file_content);
    let mut bfr = String::new();
    html::push_html(&mut bfr, parser);
    Ok(bfr)
}


enum WikiResponse {
    NamedFile(NamedFile),
    Template(Template),
}

impl WikiResponse {
    fn ok(self) -> io::Result<Self> {
        return Ok(self);
    }
}

impl<'a> rocket::response::Responder<'a> for WikiResponse {
    fn respond(self) -> Result<rocket::Response<'a>, rocket::http::Status> {
        match self {
            WikiResponse::Template(x) => x.respond(),
            WikiResponse::NamedFile(y) => y.respond(),
        }
    }
}

fn static_files(file: &PathBuf) -> Option<NamedFile> {
    let wiki_root = Path::new(r"C:\Dev\wiki");
    NamedFile::open(wiki_root.join(file))
            .or_else(|_| NamedFile::open(file))
            .ok()
}

#[get("/")]
fn index() -> Template {
    let vec = vec![1, 2, 3];
    Template::render("index", &vec)
}

#[get("/<path..>", rank=2)]
fn get(path: PathBuf) -> io::Result<WikiResponse> {
    if let Some(resp) = static_files(&path) {
        return WikiResponse::NamedFile(resp).ok();
    }

    let wiki_root = Path::new(r"C:\Dev\wiki");

    let page_name: String = path.to_str().unwrap().into();
 
    let file_name = format!("{}.md", &page_name);

    let path = wiki_root.join(file_name);
    if !path.exists() {
        let error_str = format!("The markdown file '{}' couldn't be found.", path.display());
        return Err(io::Error::new(io::ErrorKind::Other, error_str.as_str()));
    }

    let content = get_html(&path)?;
    
    let view_finder = view::ViewFinder::new(wiki_root.to_owned());
    let groups = view_finder.get_groups().expect("Unable to read wiki directory");
    let prev_next = view::find_prev_next(&groups, &page_name);
    let context = TemplateContext {
        prev_url: prev_next.prev.map_or("".into(), |p| p.file_name),
        next_url: prev_next.next.map_or("".into(), |p| p.file_name),
        title: page_name.clone(),
        page: page_name,
        view_groups: groups,
        content: content,
    };

    WikiResponse::Template(Template::render("index", &context)).ok()
}

fn main() {
    rocket::ignite().mount("/", routes![index, get]).launch();
}