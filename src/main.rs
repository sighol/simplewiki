#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate pulldown_cmark;
extern crate regex;
#[macro_use] extern crate serde_derive;

use std::io;
use std::path::{Path, PathBuf};

use std::io::prelude::*;
use std::fs::File;

use rocket_contrib::Template;
use rocket::response::NamedFile;
use rocket::response::Redirect;
use rocket::Data;

mod view;

#[derive(Serialize)]
struct ShowContext {
    view_groups: Vec<view::ViewGroup>,
    content: String,
    title: String,
    prev_url: String,
    next_url: String,
    page: String,
}

#[derive(Serialize)]
struct EditContext {
    view_groups: Vec<view::ViewGroup>,
    content: String,
    title: String,
    page: String,
}


#[derive(Serialize)]
struct IndexContent {
    title: String,
    view_groups: Vec<view::ViewGroup>,
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

fn root() -> PathBuf {
    PathBuf::from(r"C:\Dev\wiki")
}

fn get_view_groups() -> Vec<view::ViewGroup> {
    let view_finder = view::ViewFinder::new(root());
    view_finder.get_groups().expect("Unable to read wiki directory")
}

#[get("/edit/<path..>", rank = 1)]
fn edit(path: PathBuf) -> io::Result<Template> {
    let markdown = get_markdown_context(&path)?;

    let context = EditContext {
        title: markdown.title,
        page: markdown.page,
        view_groups: get_view_groups(),
        content: markdown.file_content,
    };
    
    Ok(Template::render("edit", &context))
}

#[post("/edit/<path..>", data = "<content>")]
fn edit_post(path: PathBuf, content: Data) -> io::Result<Redirect> {
    let new_content = data_to_string(content)?;
    let context = get_markdown_context(&path)?;

    println!("File path: {}", context.file_path.display());
    let mut file = File::create(context.file_path)?;
    file.write_all(new_content.as_bytes())?;
    
    let path_str = path.to_str().unwrap();
    let path_str = format!("/{}", path_str);
    let path_str = &path_str;
    println!("Path is: {}", path_str);
    Ok(Redirect::to(path_str))
}

fn data_to_string(content: Data) -> io::Result<String> {
    let mut bfr: Vec<u8> = Vec::new();
    content.stream_to(&mut bfr)?;
    String::from_utf8(bfr)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Could not parse as UTF."))
}

#[get("/")]
fn index() -> Template {
    let content = IndexContent {
        title: "Home".to_string(),
        view_groups: get_view_groups(),
    };

    Template::render("index", &content)
}

struct MarkdownContext {
    page: String,
    title: String,
    file_path: PathBuf,
    file_content: String,
}

fn get_markdown_context(path: &Path) -> io::Result<MarkdownContext> {
    let wiki_root = root();
    let wiki_root = wiki_root.as_path();

    let page_name: String = path.to_str().unwrap().to_string();
 
    let file_path = format!("{}.md", &page_name);

    let path = wiki_root.join(&file_path);
    if !path.exists() {
        let error_str = format!("The markdown file '{}' couldn't be found.", path.display());
        return Err(io::Error::new(io::ErrorKind::Other, error_str.as_str()));
    }

    let file_content = get_file_content(&path)?;

    Ok(MarkdownContext {
        page: page_name.clone(),
        title: page_name,
        file_path: path,
        file_content: file_content,
    })
}

fn get_file_content(file: &Path) -> io::Result<String> {
    let mut file = File::open(file)?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;
    Ok(file_content)
}

fn get_html(file_content: &str) -> String {
    use pulldown_cmark::{html, Parser};

    let parser = Parser::new(file_content);
    let mut bfr = String::new();
    html::push_html(&mut bfr, parser);
    bfr
}

#[get("/<path..>", rank = 2)]
fn show(path: PathBuf) -> io::Result<WikiResponse> {
    if let Some(resp) = static_files(&path) {
        return WikiResponse::NamedFile(resp).ok();
    }

    let markdown = get_markdown_context(&path)?;
    let view_groups = get_view_groups();
    let prev_next = view::find_prev_next(&view_groups, &markdown.page);

    let context = ShowContext {
        prev_url: prev_next.prev.map_or("".into(), |p| p.file_name),
        next_url: prev_next.next.map_or("".into(), |p| p.file_name),
        title: markdown.title,
        page: markdown.page,
        view_groups: view_groups,
        content: get_html(&markdown.file_content),
    };

    WikiResponse::Template(Template::render("show", &context)).ok()
}



fn main() {
    rocket::ignite().mount("/", routes![index, show, edit, edit_post]).launch();
}