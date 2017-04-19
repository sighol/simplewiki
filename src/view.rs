use std::fmt;

use std::path::{Path, PathBuf};
use std::fs;
use std::io;

use regex::Regex;

#[derive(Serialize)]
pub struct View {
    file_name: String,
    name: String,
}

#[derive(Serialize)]
pub struct ViewGroup {
    key: String,
    views: Vec<View>,
}

impl ViewGroup {
    fn new(key: &str) -> Self {
        ViewGroup {
            key: key.into(),
            views: Vec::new(),
        }
    }
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{View file_name={}}}", &self.file_name)
    }
}

pub struct ViewFinder
{
    path: PathBuf,
    page_name_regex: Regex,
}

impl ViewFinder {
    pub fn new(path: PathBuf) -> Self {
        ViewFinder {
            path: path,
            page_name_regex: Regex::new(r"(.*)\.md").unwrap(),
        }
    }

    fn get_file_name(&self, path: &Path) -> String {
        path.file_name().and_then(|name| name.to_str()).expect("Unable to find name of folder").into()
    }

    fn get_view(&self, view_group_key: &str, markdown_path: &Path) -> Option<View> {
        let file_name= self.get_file_name(markdown_path);

        if let Some(caps) = self.page_name_regex.captures(&file_name)
        {
            let key = caps.get(1).map_or("", |m| m.as_str());
            let name = format!("{}/{}", view_group_key, key);
            
            let view = View {
                name: key.into(),
                file_name: name.into(),
            };

            Some(view)
        } else {
            None
        }
    }

    fn get_group(&self, path: &Path) -> io::Result<ViewGroup> {
        let key = self.get_file_name(path);
        let mut view_group = ViewGroup::new(&key);

        for markdown_file in fs::read_dir(path)? {
            let path = markdown_file?.path();
            if let Some(view) = self.get_view(&key, &path) {
                view_group.views.push(view);
            }
        }
        Ok(view_group)
    }

    pub fn get_groups(&self) -> io::Result<Vec<ViewGroup>> {
        let dirs = fs::read_dir(&self.path)?;
        let mut view_groups = Vec::new();
        for entry in dirs {
            let path = entry?.path();
            if path.is_dir() {
                let view_group = self.get_group(&path)?;
                if view_group.views.len() > 0 {
                    view_groups.push(view_group);
                }
            }
        }

        Ok(view_groups)
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn view_format() {
        let view = View { name: "Sigurd".into(), file_name: "file".into()};
        let display = format!("{}", view);
        assert!(display == "{View file_name=file}");
    }
}