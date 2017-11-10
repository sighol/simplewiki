use std::time::Duration;
use walkdir::WalkDir;
use walkdir::DirEntry;

pub struct SearchResult {
    pub pattern: String,
    pub matches: Vec<SearchFileMatch>,
    pub elapsed: Duration,
}


pub struct SearchFileMatch {
    pub file_name: String,
    pub file_path: String,
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
    entry.file_name()
        .to_str()
        .map(|s| s.ends_with(".md"))
        .unwrap_or(false)
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
        println!("{}", entry.path().display());
    }

    result
}