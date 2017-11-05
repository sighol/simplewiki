use std::fmt;

use std::path::{Path, PathBuf};
use std::fs;
use std::io;

use regex::Regex;

#[derive(Serialize)]
pub struct View {
    pub file_name: String,
    pub name: String,
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{View file_name={}}}", &self.file_name)
    }
}

impl Clone for View {
    fn clone(&self) -> Self {
        View {
            file_name: self.file_name.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct ViewGroup {
    pub key: String,
    pub views: Vec<View>,
}

impl ViewGroup {
    fn new(key: &str) -> Self {
        ViewGroup {
            key: key.into(),
            views: Vec::new(),
        }
    }
}

pub struct ViewFinder {
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

    fn get_file_name(&self, path: &Path) -> Option<String> {
        let s = path.file_name().and_then(|name| name.to_str()).map(|str| {
            str.to_string()
        });
        s
    }

    fn get_view(&self, view_group_key: &str, markdown_path: &Path) -> Option<View> {
        let file_name = self.get_file_name(markdown_path).unwrap_or_else(
            || ".".to_string(),
        );

        if let Some(caps) = self.page_name_regex.captures(&file_name) {
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
        let key = self.get_file_name(path).unwrap_or_else(|| ".".to_string());
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

        let mut local_group = self.get_group(&self.path)?;

        {
            local_group.key = String::from("/");
            for view in &mut local_group.views {
                let mut path = PathBuf::new();
                path.push(&view.file_name);
                view.file_name = self.get_file_name(&path).unwrap_or_else(|| ".".to_string());
            }
        }

        if local_group.views.len() > 0 {
            view_groups.insert(0, local_group);
        }

        Ok(view_groups)
    }
}

#[derive(Serialize)]
pub struct PrevNextResult {
    pub prev: Option<View>,
    pub next: Option<View>,
}

impl PrevNextResult {
    fn new() -> Self {
        PrevNextResult {
            prev: None,
            next: None,
        }
    }
}

pub fn find_prev_next(view_groups: &[ViewGroup], view_name: &str) -> PrevNextResult {
    let view_name = view_name.to_string();
    let view_name = view_name.replace("\\", "/");
    let view_name: &str = &view_name;
    let views: Vec<&View> = view_groups
        .iter()
        .flat_map(|group| group.views.iter())
        .collect();

    let mut result = PrevNextResult::new();

    let mut prev = -1i32;
    let mut next = -1i32;
    let mut current = -1i32;

    for index in 0..views.len() {
        prev = current;
        current = index as i32;
        next = -1i32;

        if index + 1 < views.len() {
            next = (index + 1) as i32;
        }

        if views[index].file_name == view_name {
            break;
        }
    }

    if prev >= 0i32 {
        result.prev = Some(views[prev as usize].clone());
    }

    if next >= 0i32 {
        result.next = Some(views[next as usize].clone());
    }

    result
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn view_format() {
        let view = View {
            name: "Sigurd".into(),
            file_name: "file".into(),
        };
        let display = format!("{}", view);
        assert!(display == "{View file_name=file}");
    }

    #[test]
    fn previous_next() {
        let groups = vec![
            ViewGroup {
                key: "a".into(),
                views: vec![
                    View {
                        name: "1".into(),
                        file_name: "a/1".into(),
                    },
                    View {
                        name: "2".into(),
                        file_name: "a/2".into(),
                    },
                    View {
                        name: "3".into(),
                        file_name: "a/3".into(),
                    },
                ],
            },
            ViewGroup {
                key: "b".into(),
                views: vec![
                    View {
                        name: "4".into(),
                        file_name: "b/4".into(),
                    },
                    View {
                        name: "5".into(),
                        file_name: "b/5".into(),
                    },
                    View {
                        name: "6".into(),
                        file_name: "b/6".into(),
                    },
                ],
            },
        ];

        let res = find_prev_next(&groups, "a/3");
        assert_eq!(res.prev.map(|x| x.name), Some("2".into()));
        assert_eq!(res.next.map(|x| x.name), Some("4".into()));

        let res = find_prev_next(&groups, "b/4");
        assert_eq!(res.prev.map(|x| x.name), Some("3".into()));
        assert_eq!(res.next.map(|x| x.name), Some("5".into()));

        let res = find_prev_next(&groups, "b/6");
        assert_eq!(res.prev.map(|x| x.name), Some("5".into()));
        assert_eq!(res.next.map(|x| x.name), None);

        let res = find_prev_next(&groups, "a/1");
        assert_eq!(res.prev.map(|x| x.name), None);
        assert_eq!(res.next.map(|x| x.name), Some("2".into()));

        let res = find_prev_next(&groups, "a/2");
        assert_eq!(res.prev.map(|x| x.name), Some("1".into()));
        assert_eq!(res.next.map(|x| x.name), Some("3".into()));

        let res = find_prev_next(&groups, "a\\2");
        assert_eq!(res.prev.map(|x| x.name), Some("1".into()));
        assert_eq!(res.next.map(|x| x.name), Some("3".into()));

        let res = find_prev_next(&groups, "b/6");
        assert_eq!(res.prev.map(|x| x.name), Some("5".into()));
        assert_eq!(res.next.map(|x| x.name), None);
    }
}
