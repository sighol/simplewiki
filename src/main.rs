#![feature(plugin)]
#![feature(custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate pulldown_cmark;
extern crate regex;
extern crate clap;
#[macro_use] extern crate serde_derive;

extern crate includedir;
extern crate phf;

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
mod markdown;
mod static_file;

use markdown::MarkdownContext;
use static_file::StaticFile;

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


enum WikiResponse {
    NamedFile(NamedFile),
    Template(Template),
    Redirect(Redirect),
}

impl<'a> rocket::response::Responder<'a> for WikiResponse {
    fn respond_to(self, request: &rocket::Request) -> Result<rocket::Response<'a>, rocket::http::Status> {
        match self {
            WikiResponse::Template(x) => x.respond_to(request),
            WikiResponse::NamedFile(x) => x.respond_to(request),
            WikiResponse::Redirect(x) => x.respond_to(request),
        }
    }
}

#[get("/<path..>", rank = 2)]
fn show(path: PathBuf, config: State<Config>) -> io::Result<WikiResponse> {
    let path = {
        let osstr = path.into_os_string();
        let without_md = osstr.to_str().unwrap().replace(".md", "");
        let mut p = PathBuf::new();
        p.push(without_md);
        p
    };

    if let Some(resp) = static_files(&config.wiki_root, &path) {
        return Ok(WikiResponse::NamedFile(resp));
    }

    let markdown = MarkdownContext::new(&config.wiki_root, &path)?;
    let view_groups = get_view_groups(&config.wiki_root);

    if markdown.exists() {
        let prev_next = view::find_prev_next(&view_groups, &markdown.page);

        let context = ShowContext {
            prev_url: prev_next.prev.map_or("".into(), |p| p.file_name),
            next_url: prev_next.next.map_or("".into(), |p| p.file_name),
            content: markdown.html().unwrap(),
            title: markdown.title,
            page: markdown.page,
            view_groups: view_groups,
        };

        Ok(WikiResponse::Template(Template::render("show", &context)))
    } else {
        let mut edit_path = PathBuf::from("edit");
        edit_path.push(&path);
        Ok(WikiResponse::Redirect(redirect_to_path(&edit_path)))
    }
}

fn static_files(wiki_root: &Path, file: &Path) -> Option<NamedFile> {
    let file_path = wiki_root.join(file);
    NamedFile::open(file_path).ok()
}

fn get_view_groups(wiki_root: &Path) -> Vec<view::ViewGroup> {
    let view_finder = view::ViewFinder::new(wiki_root.to_owned());
    view_finder.get_groups().expect("Unable to read wiki directory")
}

#[derive(Serialize)]
struct EditContext {
    view_groups: Vec<view::ViewGroup>,
    content: String,
    title: String,
    page: String,
}

#[get("/edit/<path..>", rank = 1)]
fn edit(path: PathBuf, config: State<Config>) -> io::Result<Template> {
    let wiki_root = &config.wiki_root;
    let markdown = MarkdownContext::new(wiki_root, &path)?;

    let context = EditContext {
        title: markdown.title,
        page: markdown.page,
        view_groups: get_view_groups(wiki_root),
        content: markdown.file_content.unwrap_or("".to_string()),
    };

    Ok(Template::render("edit", &context))
}

#[get("/static/<path..>", rank=1)]
fn static_file(path: PathBuf) -> io::Result<StaticFile> {
    Ok(StaticFile::new(path))
}

#[derive(FromForm)]
struct EditForm {
    content: String
}

#[post("/edit/<path..>", data = "<content>")]
fn edit_post(path: PathBuf, content: Form<EditForm>, config: State<Config>) -> io::Result<Redirect> {
    let new_content = content.into_inner().content;

    let context = MarkdownContext::new(&config.wiki_root, &path)?;

    let mut file = File::create(context.file_path)?;
    file.write_all(new_content.as_bytes())?;

    Ok(redirect_to_path(&path))
}

fn redirect_to_path(path: &Path) -> Redirect {
    let path_str = path.to_str().unwrap();
    let path_str = format!("/{}", path_str);
    let path_str = &path_str;
    Redirect::to(path_str)
}

#[get("/edit_editor/<path..>", rank = 1)]
fn edit_editor(path: PathBuf, config: State<Config>) -> io::Result<Redirect> {
    use std::process::Command;

    let markdown = MarkdownContext::new(&config.wiki_root, &path)?;
    println!("Path is  {}", &markdown.file_path.display());

    let editor = &config.editor;

    Command::new(editor)
        .arg(&markdown.file_path)
        .status()?;

    Ok(redirect_to_path(&path))
}

#[derive(Serialize)]
struct IndexContent {
    title: String,
    view_groups: Vec<view::ViewGroup>,
}

#[get("/")]
fn index(config: State<Config>) -> Template {
    let content = IndexContent {
        title: "Home".to_string(),
        view_groups: get_view_groups(&config.wiki_root),
    };

    Template::render("index", &content)
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

    let port = matches.value_of("port").unwrap_or("8002");
    let wiki_root = matches.value_of("wiki_root").unwrap_or(".");
    let editor = matches.value_of("editor").unwrap_or("subl");

    let config = Config {
        editor: editor.to_string(),
        wiki_root: PathBuf::from(wiki_root),
    };

    env::set_var("ROCKET_PORT", port);
    env::set_var("ROCKET_WORKERS", "128");

    let template_dir = static_file::extract_templates();

    env::set_var("ROCKET_TEMPLATE_DIR", template_dir.to_str().unwrap());

    rocket::ignite()
        .mount("/", routes![index, show, edit, edit_post, edit_editor, static_file])
        .attach(Template::fairing())
        .manage(config)
        .launch();
}
