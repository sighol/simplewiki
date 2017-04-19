use std::fmt;

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

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{View file_name={}}}", &self.file_name)
    }
}

pub fn get_groups() -> Vec<ViewGroup> {
    vec![
        ViewGroup {
            key: "km".into(),
            views: vec![
                View {
                    name: "Aasgard".into(),
                    file_name: "km\\aasgard".into()
                },
                View {
                    name: "Aim".into(),
                    file_name: "km\\aim".into(),
                }
            ],
        }
    ]
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