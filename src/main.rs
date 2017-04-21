#![feature(plugin)]
#![feature(custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate pulldown_cmark;
extern crate regex;
extern crate clap;
#[macro_use] extern crate serde_derive;

use std::io;
use std::path::{Path, PathBuf};

use std::io::prelude::*;
use std::fs::File;

use rocket_contrib::Template;
use rocket::response::NamedFile;
use rocket::response::Redirect;
use rocket::request::Form;
use rocket::State;

mod view;

struct Config {
    editor: String,
    wiki_root: PathBuf,
}

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

fn get_view_groups(wiki_root: &Path) -> Vec<view::ViewGroup> {
    let view_finder = view::ViewFinder::new(wiki_root.to_owned());
    view_finder.get_groups().expect("Unable to read wiki directory")
}

#[get("/edit_editor/<path..>", rank = 1)]
fn edit_editor(path: PathBuf, config: State<Config>) -> io::Result<Redirect> {
    use std::process::Command;

    let markdown = get_markdown_context(&config.wiki_root, &path)?;
    println!("Path is  {}", &markdown.file_path.display());

    let editor = &config.editor;

    Command::new(editor)
        .arg(&markdown.file_path)
        .status()?;

    Ok(redirect_to_path(&path))
}

fn redirect_to_path(path: &Path) -> Redirect {
    let path_str = path.to_str().unwrap();
    let path_str = format!("/{}", path_str);
    let path_str = &path_str;
    Redirect::to(path_str)
}

#[get("/edit/<path..>", rank = 1)]
fn edit(path: PathBuf, config: State<Config>) -> io::Result<Template> {
    let wiki_root = &config.wiki_root;
    let markdown = get_markdown_context(wiki_root, &path)?;

    let context = EditContext {
        title: markdown.title,
        page: markdown.page,
        view_groups: get_view_groups(wiki_root),
        content: markdown.file_content,
    };

    Ok(Template::render("edit", &context))
}

#[derive(FromForm)]
struct EditForm {
    content: String
}

#[post("/edit/<path..>", data = "<content>")]
fn edit_post(path: PathBuf, content: Form<EditForm>, config: State<Config>) -> io::Result<Redirect> {
    let new_content = content.into_inner().content;

    let wiki_root = &config.wiki_root;
    let context = get_markdown_context(wiki_root, &path)?;

    println!("File path: {}", context.file_path.display());
    let mut file = File::create(context.file_path)?;
    file.write_all(new_content.as_bytes())?;

    Ok(redirect_to_path(&path))
}

#[get("/")]
fn index(config: State<Config>) -> Template {
    let content = IndexContent {
        title: "Home".to_string(),
        view_groups: get_view_groups(&config.wiki_root),
    };

    Template::render("index", &content)
}

struct MarkdownContext {
    page: String,
    title: String,
    file_path: PathBuf,
    file_content: String,
}

fn get_markdown_context(wiki_root: &Path, path: &Path) -> io::Result<MarkdownContext> {
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
fn show(path: PathBuf, config: State<Config>) -> io::Result<WikiResponse> {
    if let Some(resp) = static_files(&config.wiki_root, &path) {
        return WikiResponse::NamedFile(resp).ok();
    }
    
    let markdown = get_markdown_context(&config.wiki_root, &path)?;
    let view_groups = get_view_groups(&config.wiki_root);
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

fn static_files(wiki_root: &Path, file: &Path) -> Option<NamedFile> {
    let file_path = wiki_root.join(file);
    NamedFile::open(file_path)
            .or_else(|_| NamedFile::open(file))
            .ok()
}

fn main() {
    use std::env;
    use clap::{Arg, App};

    let matches = App::new("simplewiki")
            .version(env!("CARGO_PKG_VERSION"))
            .author(env!("CARGO_PKG_AUTHORS"))
            .arg(Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .takes_value(true))
            .arg(Arg::with_name("wiki_root")
                    .long("wiki-root")
                    .value_name("ROOT")
                    .takes_value(true))
            .arg(Arg::with_name("editor")
                    .long("editor")
                    .takes_value(true))
            .get_matches();

    println!("Args: {:?}", matches);

    let port = matches.value_of("port").unwrap_or("8002");
    let wiki_root = matches.value_of("wiki_root").unwrap_or(".");
    let editor = matches.value_of("editor").unwrap_or("subl");

    let config = Config {
        editor: editor.to_string(),
        wiki_root: PathBuf::from(wiki_root),
    };

    env::set_var("ROCKET_PORT", port);
    env::set_var("ROCKET_WORKERS", "128");
    rocket::ignite()
        .mount("/", routes![index, show, edit, edit_post, edit_editor])
        .manage(config)
        .launch();
}