use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;
use std::path::{Path, PathBuf};
use tera::escape_html;

use walkdir::{WalkDir, DirEntry};

use stopwatch::Stopwatch;

use regex;

use errors::*;

const CONTEXT: usize = 3;

#[derive(Serialize)]
pub struct SearchResult {
    pub pattern: String,
    pub matches: Vec<SearchFileMatch>,
    pub elapsed: i64,
}

#[derive(Serialize)]
pub struct SearchFileMatch {
    pub file_name: String,
    pub file_path: PathBuf,
    pub url: String,
    pub contexts: Vec<SearchFileMatchContext>,
}

#[derive(Serialize)]
pub struct SearchFileMatchContext {
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

impl ToHtml for [SearchMatchContext] {
    fn to_html(&self) -> String {
        let mut lines = vec![];
        lines.push("<table>".to_string());
        for m in self {
            let has_match = m.lines.iter().any(|c| match c {
                &SearchMatchText::Match(_) => true,
                _ => false,
            });

            if has_match {
                lines.push("<tr class=\"line-match\">".to_string());
            } else {
                lines.push("<tr>".to_string());
            }

            lines.push(format!(
                "\t<td class=\"line-number\">{}</td>",
                m.line_number
            ));

            lines.push("<td class=\"line\">".to_string());

            let match_segments: Vec<String> = m.lines
                .iter()
                .map(|c| match c {
                    &SearchMatchText::Text(ref string) => escape_html(&string),
                    &SearchMatchText::Match(ref string) => {
                        format!("<span class=\"match\">{}</span>", escape_html(&string))
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
    if entry.file_type().is_dir() {
        true
    } else {
        entry
            .file_name()
            .to_str()
            .map(|s| s.ends_with(".md"))
            .unwrap_or(false)
    }
}

pub fn search<F>(pattern: &str, directory: &str, url: F) -> Result<SearchResult>
where
    F: Fn(&Path, &Path) -> Result<String>,
{
    let sw = Stopwatch::start_new();
    let mut result = SearchResult {
        pattern: pattern.to_string(),
        matches: vec![],
        elapsed: 0,
    };

    let walker = WalkDir::new(directory).into_iter();
    for entry in walker.filter_entry(|e| is_markdown(e)) {
        let entry: DirEntry = entry.chain_err(
            || "unable to unwrap entry. Directory traversel error?",
        )?;

        if entry.metadata().map(|e| e.is_dir()).unwrap_or(false) {
            continue;
        }

        if let Ok(search_file_match) = search_file(entry, pattern, directory, &url) {
            result.matches.push(search_file_match);
        }
    }

    result.elapsed = sw.elapsed_ms();

    Ok(result)
}

fn search_file<F>(
    entry: DirEntry,
    pattern: &str,
    directory: &str,
    url: &F,
) -> Result<SearchFileMatch>
where
    F: Fn(&Path, &Path) -> Result<String>,
{
    let pattern = &format!("(?i){}", pattern);
    let pattern_re = regex::Regex::new(pattern).chain_err(|| "Invalid pattern")?;
    let pattern_specific_re =
        regex::Regex::new(&format!("^(?P<pre>.*)(?P<match>{})(?P<post>.*)$", pattern)).unwrap();

    let f = File::open(entry.path()).chain_err(|| "Failed to open file")?;

    let file = BufReader::new(&f);

    let lines: Vec<String> = file.lines().filter_map(|e| e.ok()).collect();

    let directory_path = Path::new(directory);
    let url = url(&directory_path, entry.path())?;
    let mut file_match = SearchFileMatch {
        file_name: entry.path().as_os_str().to_str().unwrap().to_string(),
        file_path: entry.path().into(),
        url: url,
        contexts: vec![],
    };

    for i in 0..lines.len() {
        let line = &lines[i];

        let is_match = pattern_re.is_match(&line);
        if !is_match {
            continue;
        }

        let captures = pattern_specific_re.captures(&line).unwrap();
        let pre = captures.name("pre").unwrap();
        let match_ = captures.name("match").unwrap();
        let post = captures.name("post").unwrap();

        let mut contexts = vec![];
        let start_index = if i > CONTEXT { i - CONTEXT } else { 0 };

        for j in start_index..i {
            let line = lines[j].to_string();
            let search_match = SearchMatchContext {
                line_number: j as i32 + 1,
                lines: vec![SearchMatchText::Text(line)],
            };

            contexts.push(search_match);
        }

        let m = SearchMatchContext {
            line_number: i as i32 + 1,
            lines: vec![
                SearchMatchText::Text(pre.as_str().into()),
                SearchMatchText::Match(match_.as_str().into()),
                SearchMatchText::Text(post.as_str().into()),
            ],
        };

        contexts.push(m);

        let end_index = if i + CONTEXT + 1 > lines.len() {
            lines.len()
        } else {
            i + CONTEXT + 1
        };
        for j in i + 1..end_index {
            let line = lines[j].to_string();
            let search_match = SearchMatchContext {
                line_number: j as i32 + 1,
                lines: vec![SearchMatchText::Text(line)],
            };

            contexts.push(search_match);
        }

        let context = SearchFileMatchContext {
            html: contexts.to_html(),
            contexts: contexts,
        };

        file_match.contexts.push(context);
    }

    if file_match.contexts.len() > 0 {
        return Ok(file_match);
    }

    bail!("No match");
}
