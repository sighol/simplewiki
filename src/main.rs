#![feature(plugin)]
#![feature(custom_derive)]
#![plugin(rocket_codegen)]

// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

extern crate rocket;
extern crate rocket_contrib;
extern crate pulldown_cmark;
extern crate regex;
extern crate clap;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate error_chain;

extern crate includedir;
extern crate phf;
extern crate ws;
extern crate spmc;
extern crate open;
extern crate notify;
extern crate walkdir;

use std::io;
use std::path::{Path, PathBuf};

use std::io::prelude::*;
use std::fs;
use std::fs::File;

use rocket_contrib::Template;
use rocket::response::NamedFile;
use rocket::response::Redirect;
use rocket::request::Form;
use rocket::State;
use rocket::config::{Config, Environment};

mod view;
mod markdown;
mod static_file;
mod refresh_socket;
mod dispatch;
mod free_port;
mod search;

use markdown::MarkdownContext;
use static_file::StaticFile;

mod errors {
    error_chain!{}
}

use errors::*;

struct SiteConfig {
    editor: String,
    wiki_root: PathBuf,
    socket_port: u16,
}


#[derive(Serialize)]
struct ShowContext {
    view_groups: Vec<view::ViewGroup>,
    content: String,
    title: String,
    prev_url: String,
    next_url: String,
    page: String,
    socket_port: u16,
}


enum WikiResponse {
    NamedFile(NamedFile),
    Template(Template),
    Redirect(Redirect),
}

impl<'a> rocket::response::Responder<'a> for WikiResponse {
    fn respond_to(
        self,
        request: &rocket::Request,
    ) -> std::result::Result<rocket::Response<'a>, rocket::http::Status> {
        match self {
            WikiResponse::Template(x) => x.respond_to(request),
            WikiResponse::NamedFile(x) => x.respond_to(request),
            WikiResponse::Redirect(x) => x.respond_to(request),
        }
    }
}

#[get("/markdown/<path..>")]
fn get_markdown(path: PathBuf, config: State<SiteConfig>) -> io::Result<String> {
    let path = path_no_markdown(path);

    let markdown = MarkdownContext::new(&config.wiki_root, &path)?;
    markdown.html().ok_or(io::Error::new(
        io::ErrorKind::Other,
        "No markdown exists for this page...",
    ))
}

fn path_no_markdown(path: PathBuf) -> PathBuf {
    let osstr = path.into_os_string();
    let without_md = osstr.to_str().unwrap().replace(".md", "");
    let mut p = PathBuf::new();
    p.push(without_md);
    p
}

#[get("/<path..>", rank = 2)]
fn show(path: PathBuf, config: State<SiteConfig>) -> io::Result<WikiResponse> {
    let path = path_no_markdown(path);

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
            socket_port: config.socket_port,
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
    view_finder.get_groups().expect(
        "Unable to read wiki directory",
    )
}

#[derive(Serialize)]
struct EditContext {
    view_groups: Vec<view::ViewGroup>,
    content: String,
    title: String,
    page: String,
}

#[get("/edit/<path..>", rank = 1)]
fn edit(path: PathBuf, config: State<SiteConfig>) -> io::Result<Template> {
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

#[get("/static/<path..>", rank = 1)]
fn static_file(path: PathBuf) -> io::Result<StaticFile> {
    Ok(StaticFile::new(path))
}

#[derive(FromForm)]
struct EditForm {
    content: String,
}

#[post("/edit/<path..>", data = "<content>")]
fn edit_post(
    path: PathBuf,
    content: Form<EditForm>,
    config: State<SiteConfig>,
) -> io::Result<Redirect> {
    let new_content = content.into_inner().content;

    let context = MarkdownContext::new(&config.wiki_root, &path)?;

    if let Some(context_folder) = Path::new(&context.file_path).parent() {
        fs::create_dir_all(context_folder)?;
    }

    let mut file = File::create(&context.file_path)?;
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
fn edit_editor(path: PathBuf, config: State<SiteConfig>) -> io::Result<Redirect> {
    use std::process::Command;

    let markdown = MarkdownContext::new(&config.wiki_root, &path)?;

    let editor = &config.editor;

    Command::new(editor).arg(&markdown.file_path).status()?;

    Ok(redirect_to_path(&path))
}

#[derive(Serialize)]
struct IndexContent {
    title: String,
    view_groups: Vec<view::ViewGroup>,
}

#[get("/")]
fn index(config: State<SiteConfig>) -> Template {
    let content = IndexContent {
        title: "Home".to_string(),
        view_groups: get_view_groups(&config.wiki_root),
    };

    Template::render("index", &content)
}

#[derive(FromForm)]
struct SearchQuery {
    pattern: String,
}

#[derive(Serialize)]
struct SearchResult {
    result: search::SearchResult,
}

#[get("/search?<query>")]
fn search(query: SearchQuery, config: State<SiteConfig>) -> errors::Result<Template> {
    let pattern = query.pattern.as_str();
    let dir = config.wiki_root.as_os_str().to_str().unwrap().to_string();
    let result: search::SearchResult = search::search(pattern, &dir, get_page_url).chain_err(|| "Search failed")?;
    let result = SearchResult { result };
    Ok(Template::render("search-result", &result))
}

fn get_page_url(file_path: &Path, wiki_root: &Path) -> Result<String> {
    let relative_path: &Path = wiki_root.strip_prefix(file_path)
        .chain_err(|| "Could not get relative path for result" )?;

    let relative_path = relative_path.as_os_str().to_str().unwrap().to_string();

    let path = relative_path.replace("\\", "/");
    let path = path.replace(".md", "");
    Ok(path)
}

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let error = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(error);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(error);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(error);
        }

        ::std::process::exit(1);
    }
}

fn run() -> Result<()> {
    use clap::{Arg, App};

    let matches = App::new("simplewiki")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("address")
                .short("a")
                .long("address")
                .value_name("ADDRESS")
                .takes_value(true)
                .help(
                    "The address you want the server to serve on. Default: localhost",
                ),
        )
        .arg(
            Arg::with_name("wiki_root")
                .index(1)
                .takes_value(true)
                .help("Directory to serve. Default: ."),
        )
        .arg(
            Arg::with_name("editor")
                .long("editor")
                .short("e")
                .takes_value(true)
                .help("Defaults to subl"),
        )
        .arg(
            Arg::with_name("skip_websocket")
                .long("no-auto-refresh")
                .help("Don't start websocket with refresh-ability"),
        )
        .arg(Arg::with_name("skip_open").long("skip-open").help(
            "Don't open the wiki page in your web browser at startup",
        ))
        .arg(Arg::with_name("verbose").long("verbose").short("v"))
        .get_matches();

    let wiki_root = matches.value_of("wiki_root").unwrap_or(".");
    let editor = matches.value_of("editor").unwrap_or("subl");
    let show_web_page = !matches.is_present("skip_open");
    let start_websocket = !matches.is_present("skip_websocket");
    let address = matches.value_of("address").unwrap_or("localhost");
    let verbose = matches.is_present("verbose");

    let port = if let Some(port_value) = matches.value_of("port") {
        port_value.parse::<u16>().chain_err(
            || "Input port was not of uint",
        )?
    } else {
        free_port::get_free_port().chain_err(
            || "Couldn't find free port for rocket",
        )?
    };

    let config = SiteConfig {
        editor: editor.to_string(),
        wiki_root: PathBuf::from(wiki_root),
        socket_port: free_port::get_free_port().chain_err(
            || "Couldn't find free port for web socket",
        )?,
    };

    let template_dir = static_file::extract_templates();

    if show_web_page {
        let path =
            format!(
            r"http://{}:{}",
            address,
            port,
        );
        open::that(&path).chain_err(
            || "Could not open page in browser",
        )?;
    }

    if start_websocket {
        refresh_socket::listen(config.socket_port, wiki_root, verbose);
    }

    let mut rocket_config = Config::build(Environment::Development)
        .address(address)
        .port(port)
        .workers(128)
        .unwrap();

    use rocket::config::Value;
    rocket_config.extras.insert(
        String::from("template_dir"),
        Value::String(
            template_dir
                .to_str()
                .chain_err(|| "Unable to open template_dir")?
                .to_string(),
        ),
    );

    rocket::custom(rocket_config, verbose)
        .mount(
            "/",
            routes![
                index,
                search,
                show,
                get_markdown,
                edit,
                edit_post,
                edit_editor,
                static_file,
            ],
        )
        .attach(Template::fairing())
        .manage(config)
        .launch();

    Ok(())
}
