#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate pulldown_cmark;
#[macro_use] extern crate serde_derive;

use std::io::prelude::*;
use std::fs::File;

use pulldown_cmark::{html, Parser};
use rocket_contrib::Template;

#[derive(Serialize)]
struct TemplateContext {
    title: String,
    content: String,
}

#[get("/hello/<name>/<age>")]
fn hello(name: &str, age: u8) -> String {

    let parser = Parser::new("# Hello World\n\n- Test\n-Test2");
    let mut bfr = String::new();
    html::push_html(&mut bfr, parser);

    format!("Hello, {} year old named {}!\n{}", age, name, &bfr)
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

#[get("/")]
fn get() -> Template {
    let file = r"C:\Dev\wiki\todo.md";
    
    let content = get_html(file);
    
    let context = TemplateContext {
        title: "My title".into(),
        content: content,
    };

    let template = "test";
    println!("Opening template {}", template);
    Template::render(template, &context)
}

fn main() {
    rocket::ignite().mount("/", routes![hello, get]).launch();
}