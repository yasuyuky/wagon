use std::path::{Path, PathBuf};

pub(crate) fn sanitize_output(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\x1b' => out.push_str("\\x1b"),
            '\x07' => out.push_str("\\x07"),
            '\x08' => out.push_str("\\x08"),
            '\x0c' => out.push_str("\\x0c"),
            '\x7f' => out.push_str("\\x7f"),
            ch if ('\u{80}'..='\u{9f}').contains(&ch) => {
                out.push_str(&format!("\\u{{{:x}}}", ch as u32));
            }
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) fn sanitize_display(text: &str) -> String {
    sanitize_output(text)
        .chars()
        .fold(String::new(), |mut out, ch| {
            match ch {
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                ch if ch < ' ' => out.push_str(&format!("\\x{:02x}", ch as u32)),
                _ => out.push(ch),
            }
            out
        })
}

pub(crate) fn display_path(path: &Path) -> String {
    sanitize_display(&path.to_string_lossy())
}

#[derive(Debug, Clone)]
pub struct Link {
    pub source: PathBuf,
    pub target: PathBuf,
    pub is_dir: bool,
}

impl Link {
    pub fn new(source: PathBuf, target: PathBuf, is_dir: bool) -> Self {
        Self {
            source,
            target,
            is_dir,
        }
    }
}

impl std::fmt::Display for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} -> {}",
            display_path(&self.target),
            display_path(&self.source)
        )
    }
}

pub enum Content {
    Text(Vec<String>),
    Binary(usize, Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_display_sanitizes_control_chars() {
        let link = Link::new(
            PathBuf::from("src\x1b]2;owned\x07\tname"),
            PathBuf::from("dst\nbad\rname"),
            false,
        );

        assert_eq!(
            format!("{link}"),
            "dst\\nbad\\rname -> src\\x1b]2;owned\\x07\\tname"
        );
    }

    #[test]
    fn output_sanitization_preserves_lines() {
        assert_eq!(
            sanitize_output("first\nsecond\x1b]2;owned\x07"),
            "first\nsecond\\x1b]2;owned\\x07"
        );
    }
}
