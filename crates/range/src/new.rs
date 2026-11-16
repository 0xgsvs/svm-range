use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{artifact::workspace_root, error::RangeError};

const TEMPLATE_DIR: &str = "_template";
const TEXT_EXTENSIONS: &[&str] = &["rs", "toml", "md", "txt", "json", "yml", "yaml"];

pub struct NewChallenge {
    pub path: PathBuf,
    pub files: Vec<PathBuf>,
}

pub fn create_from_template(slug: &str) -> Result<NewChallenge, RangeError> {
    if !is_valid_slug(slug) {
        return Err(RangeError::InvalidSlug(slug.to_owned()));
    }

    let root = workspace_root()?;
    let template = root.join("challenges").join(TEMPLATE_DIR);
    let destination = root.join("challenges").join(slug);
    if destination.exists() {
        return Err(RangeError::AlreadyExists(slug.to_owned(), destination));
    }

    let title = title_case(slug);
    let mut files = Vec::new();
    copy_tree(&template, &destination, slug, &title, &mut files)?;

    Ok(NewChallenge {
        path: destination,
        files,
    })
}

fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 48
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

fn title_case(slug: &str) -> String {
    slug.split('-')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => {
                    let mut title = String::new();
                    title.push(first.to_ascii_uppercase());
                    title.push_str(characters.as_str());
                    title
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn copy_tree(
    from: &Path,
    to: &Path,
    slug: &str,
    title: &str,
    files: &mut Vec<PathBuf>,
) -> Result<(), RangeError> {
    fs::create_dir_all(to).map_err(|source| RangeError::CreateDir {
        path: to.to_path_buf(),
        source,
    })?;

    let entries = fs::read_dir(from).map_err(|source| RangeError::Read {
        path: from.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| RangeError::Read {
            path: from.to_path_buf(),
            source,
        })?;
        let name = entry.file_name();
        if name == "target" || name == ".git" || name == "Cargo.lock" {
            continue;
        }

        let source_path = entry.path();
        let target_path = to.join(&name);
        let file_type = entry.file_type().map_err(|source| RangeError::Metadata {
            path: source_path.clone(),
            source,
        })?;

        if file_type.is_dir() {
            copy_tree(&source_path, &target_path, slug, title, files)?;
        } else if file_type.is_file() {
            let bytes = fs::read(&source_path).map_err(|source| RangeError::Read {
                path: source_path.clone(),
                source,
            })?;
            let rendered = if is_text(&source_path) {
                substitute(&String::from_utf8_lossy(&bytes), slug, title).into_bytes()
            } else {
                bytes
            };
            fs::write(&target_path, rendered).map_err(|source| RangeError::Write {
                path: target_path.clone(),
                source,
            })?;
            files.push(target_path);
        }
    }

    Ok(())
}

fn is_text(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| TEXT_EXTENSIONS.contains(&extension))
}

fn substitute(text: &str, slug: &str, title: &str) -> String {
    let snake = slug.replace('-', "_");
    let mut rendered = text.to_owned();
    rendered = rendered.replace("__SLUG_SNAKE__", &snake);
    rendered = rendered.replace("__PKG_VULNERABLE__", &format!("{slug}-vulnerable"));
    rendered = rendered.replace("__PKG_PATCHED__", &format!("{slug}-patched"));
    rendered = rendered.replace("__LIB_VULNERABLE__", &format!("{snake}_vulnerable"));
    rendered = rendered.replace("__LIB_PATCHED__", &format!("{snake}_patched"));
    rendered = rendered.replace("__SLUG__", slug);
    rendered = rendered.replace("__TITLE__", title);
    rendered = rendered.replace(
        "__DESCRIPTION__",
        "TODO: describe the bug in one line for `svm-range list`.",
    );
    rendered
}
