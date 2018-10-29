use std::io;
use std::path::{Path, PathBuf};

use std::fs::File;
use std::io::prelude::*;

pub struct MarkdownContext {
    pub page: String,
    pub title: String,
    pub file_path: PathBuf,
    pub file_content: Option<String>,
}

impl MarkdownContext {
    pub fn new(wiki_root: &Path, path: &Path) -> io::Result<Self> {
        let page_name: String = path.to_str().unwrap().to_string();
        let file_path = format!("{}.md", &page_name);

        let path = wiki_root.join(&file_path);

        let file_content = if path.exists() {
            Some(get_file_content(&path)?)
        } else {
            None
        };

        Ok(MarkdownContext {
            page: page_name.clone(),
            title: page_name,
            file_path: path,
            file_content: file_content,
        })
    }

    pub fn html(&self) -> Option<String> {
        use pulldown_cmark::{html, Options, Parser};

        if let Some(ref file_content) = self.file_content {
            let parser = Parser::new_ext(&file_content, Options::all());
            let mut bfr = String::new();
            html::push_html(&mut bfr, parser);
            Some(bfr)
        } else {
            None
        }
    }

    pub fn exists(&self) -> bool {
        self.file_path.exists()
    }
}

fn get_file_content(file: &Path) -> io::Result<String> {
    let mut file = File::open(file)?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;
    Ok(file_content)
}
