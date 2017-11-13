use std::time::Duration;
use walkdir::WalkDir;
use walkdir::DirEntry;

use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;
use std::path::{Path, PathBuf};

use regex;

pub struct SearchResult {
    pub pattern: String,
    pub matches: Vec<SearchFileMatch>,
    pub elapsed: Duration,
}


pub struct SearchFileMatch {
    pub file_name: String,
    pub file_path: PathBuf,
    pub contextes: Vec<SearchMatchContext>,
}

pub struct SearchMatchContext {
    pub lines_before: Vec<String>,
    pub text_before: String,
    pub match_value: String,
    pub text_after: String,
    pub lines_after: Vec<String>,
}

fn is_markdown(entry: &DirEntry) -> bool {
    if entry.metadata().map(|e| e.is_dir()).unwrap_or(false) {
        true
    } else {
        entry
            .file_name()
            .to_str()
            .map(|s| s.ends_with(".md"))
            .unwrap_or(false)
    }
}

pub fn search(pattern: &str, directory: &str) -> SearchResult {
    let context = 4;

    let mut result = SearchResult {
        pattern: pattern.to_string(),
        matches: vec![],
        elapsed: Duration::new(0, 0),
    };

    let walker = WalkDir::new(directory).into_iter();
    for entry in walker.filter_entry(|e| is_markdown(e)) {
        let entry = entry.unwrap();

        if entry.metadata().map(|e| e.is_dir()).unwrap_or(false) {
            continue;
        }

        println!("{}", entry.path().display());

        let re = regex::Regex::new(pattern).unwrap();

        if let Ok(f) = File::open(entry.path()) {
            let mut file = BufReader::new(&f);
            for line in file.lines() {
                if let Ok(line) = line {
                    let is_match = re.is_match(&line);
                    if is_match {
                        println!("LINE: {}", &line);
                        let m = SearchMatchContext {
                            lines_before: vec![],
                            text_before: "".into(),
                            match_value: line,
                            text_after: "".into(),
                            lines_after: vec![],
                        };
                        let fm = SearchFileMatch {
                            file_name: "".into(),
                            file_path: entry.path().into(),
                            contextes: vec![m],
                        };

                        result.matches.push(fm);
                    }
                }
            }
        }
    }

    result
}
