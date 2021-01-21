use std::collections::HashSet;
use std::path::PathBuf;

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
            self.target.to_str().unwrap_or_default(),
            self.source.to_str().unwrap_or_default()
        )
    }
}

pub enum Content {
    Text(Vec<String>),
    Binary(usize, Vec<u8>),
}

pub struct PathDict {
    pub dir: HashSet<PathBuf>,
    pub ign: HashSet<PathBuf>,
}
