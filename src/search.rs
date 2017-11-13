use std::time::Duration;
use walkdir::WalkDir;
use walkdir::DirEntry;

use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;
use std::path::{Path, PathBuf};

use regex;

use errors::*;

#[derive(Serialize)]
pub struct SearchResult {
    pub pattern: String,
    pub matches: Vec<SearchFileMatch>,
    pub elapsed: Duration,
}

#[derive(Serialize)]
pub struct SearchFileMatch {
    pub file_name: String,
    pub file_path: PathBuf,
    pub contexts: Vec<SearchMatchContext>,
    pub html: String,
}

#[derive(Serialize)]
pub struct SearchMatchContext {
    pub line_number: i32,
    pub lines: Vec<SearchMatchText>,
}

#[derive(Serialize)]
pub enum SearchMatchText {
    Text(String),
    Match(String),
}

trait ToHtml {
    fn to_html(&self) -> String;
}

impl ToHtml for Vec<SearchMatchContext> {
    fn to_html(&self) -> String {
        let mut lines = vec![];
        lines.push("<table>".to_string());
        for m in self {
            lines.push("<tr>".to_string());
            lines.push(format!(
                "\t<td class=\"line-number\">{}</td>",
                m.line_number
            ));
            lines.push("<td class=\"line\">".to_string());

            let match_segments: Vec<String> = m.lines
                .iter()
                .map(|c| match c {
                    &SearchMatchText::Text(ref string) => string.to_string(),
                    &SearchMatchText::Match(ref string) => {
                        format!("<span class=\"match\">{}</span>", string)
                    }
                })
                .collect();

            lines.push(match_segments.join(""));

            lines.push("</td>".to_string());
            lines.push("</tr>".to_string());
        }

        lines.push("</table>".to_string());
        lines.join("\n")
    }
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

pub fn search(pattern: &str, directory: &str) -> Result<SearchResult> {
    let context = 3;

    let mut result = SearchResult {
        pattern: pattern.to_string(),
        matches: vec![],
        elapsed: Duration::new(0, 0),
    };

    let walker = WalkDir::new(directory).into_iter();
    for entry in walker.filter_entry(|e| is_markdown(e)) {
        let entry: DirEntry = entry.chain_err(
            || "unable to unwrap entry. Directory traversel error?",
        )?;

        if entry.metadata().map(|e| e.is_dir()).unwrap_or(false) {
            continue;
        }

        println!("{}", entry.path().display());

        let re = regex::Regex::new(pattern).chain_err(|| "Invalid pattern")?;

        let re2 = regex::Regex::new(&format!("^(?P<pre>.*)(?P<match>{})(?P<post>.*)$", pattern))
            .unwrap();

        if let Ok(f) = File::open(entry.path()) {
            let mut file = BufReader::new(&f);

            let lines: Vec<String> = file.lines().filter_map(|e| e.ok()).collect();

            for i in 0..lines.len() {
                let line = &lines[i];

                let is_match = re.is_match(&line);
                if is_match {
                    let captures = re2.captures(&line).unwrap();
                    let pre = captures.name("pre").unwrap();
                    let match_ = captures.name("match").unwrap();
                    let post = captures.name("post").unwrap();

                    let mut contexts = vec![];
                    let start_index = if i > context { i - context } else { 0 };

                    for j in start_index..i {
                        let line = lines[j].to_string();
                        let search_match = SearchMatchContext {
                            line_number: j as i32 + 1,
                            lines: vec![SearchMatchText::Text(line)],
                        };

                        contexts.push(search_match);
                    }

                    println!("LINE: {}", &line);
                    let m = SearchMatchContext {
                        line_number: i as i32 + 1,
                        lines: vec![
                            SearchMatchText::Text(pre.as_str().into()),
                            SearchMatchText::Match(match_.as_str().into()),
                            SearchMatchText::Text(post.as_str().into()),
                        ],
                    };

                    contexts.push(m);

                    let end_index = if i + context + 1 > lines.len() {
                        lines.len()
                    } else {
                        i + context + 1
                    };
                    for j in i + 1..end_index {
                        let line = lines[j].to_string();
                        let search_match = SearchMatchContext {
                            line_number: j as i32 + 1,
                            lines: vec![SearchMatchText::Text(line)],
                        };

                        contexts.push(search_match);
                    }

                    let html = contexts.to_html();
                    let fm = SearchFileMatch {
                        file_name: entry.path().as_os_str().to_str().unwrap().to_string(),
                        file_path: entry.path().into(),
                        contexts,
                        html,
                    };


                    result.matches.push(fm);
                }

            }
        }
    }

    Ok(result)
}
