use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, OnceLock, PoisonError},
    time::SystemTime,
};

use crate::{
    Mode,
    error::{BuildError, RangeError},
};

type ArtifactKey = (String, Mode);

static ARTIFACTS: OnceLock<Mutex<HashMap<ArtifactKey, PathBuf>>> = OnceLock::new();
static BUILD_LOCK: Mutex<()> = Mutex::new(());

fn artifacts() -> &'static Mutex<HashMap<ArtifactKey, PathBuf>> {
    ARTIFACTS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn workspace_root() -> Result<PathBuf, RangeError> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| RangeError::WorkspaceRoot(manifest_dir.to_path_buf()))?;
    fs::canonicalize(root).map_err(|source| RangeError::Metadata {
        path: root.to_path_buf(),
        source,
    })
}

pub fn manifest_path(slug: &str, mode: Mode) -> Result<PathBuf, RangeError> {
    Ok(workspace_root()?
        .join("challenges")
        .join(slug)
        .join(mode.as_str())
        .join("Cargo.toml"))
}

pub fn artifact_path(slug: &str, mode: Mode) -> Result<PathBuf, RangeError> {
    let library = format!("{}_{}", slug.replace('-', "_"), mode.as_str());
    Ok(workspace_root()?
        .join("target")
        .join("sbf")
        .join(slug)
        .join(mode.as_str())
        .join(library)
        .with_extension("so"))
}

pub fn ensure_artifact(slug: &str, mode: Mode) -> Result<PathBuf, RangeError> {
    let key = (slug.to_owned(), mode);

    if let Some(cached) = artifacts()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
        .cloned()
        && cached.is_file()
    {
        return Ok(cached);
    }

    // Serialize compilations: each SBF build is memory hungry, and a shared
    // target directory means concurrent cargo invocations would only queue.
    let _build_guard = BUILD_LOCK.lock().unwrap_or_else(PoisonError::into_inner);

    let artifact = artifact_path(slug, mode)?;
    let manifest = manifest_path(slug, mode)?;
    let crate_dir = manifest
        .parent()
        .ok_or_else(|| RangeError::WorkspaceRoot(manifest.clone()))?;

    let rebuild = match fs::metadata(&artifact) {
        Ok(metadata) => {
            let artifact_time = metadata.modified().map_err(|source| RangeError::Metadata {
                path: artifact.clone(),
                source,
            })?;
            newest_source_time(crate_dir)? > artifact_time
        }
        Err(_) => true,
    };

    if rebuild {
        build(slug, mode, crate_dir, &artifact)?;
    }

    if !artifact.is_file() {
        return Err(RangeError::MissingArtifact(artifact));
    }

    artifacts()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, artifact.clone());
    Ok(artifact)
}

fn newest_source_time(crate_dir: &Path) -> Result<SystemTime, RangeError> {
    let manifest = crate_dir.join("Cargo.toml");
    let mut newest = modified(&manifest)?;
    walk_newest(&crate_dir.join("src"), &mut newest)?;
    Ok(newest)
}

fn walk_newest(directory: &Path, newest: &mut SystemTime) -> Result<(), RangeError> {
    let entries = fs::read_dir(directory).map_err(|source| RangeError::Metadata {
        path: directory.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| RangeError::Metadata {
            path: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            walk_newest(&path, newest)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let time = modified(&path)?;
            if time > *newest {
                *newest = time;
            }
        }
    }
    Ok(())
}

fn modified(path: &Path) -> Result<SystemTime, RangeError> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|source| RangeError::Metadata {
            path: path.to_path_buf(),
            source,
        })
}

fn build(slug: &str, mode: Mode, crate_dir: &Path, artifact: &Path) -> Result<(), RangeError> {
    let root = workspace_root()?;
    let out_dir = artifact
        .parent()
        .ok_or_else(|| RangeError::WorkspaceRoot(artifact.to_path_buf()))?;
    fs::create_dir_all(out_dir).map_err(|source| RangeError::CreateDir {
        path: out_dir.to_path_buf(),
        source,
    })?;

    let manifest = crate_dir.join("Cargo.toml");
    let target_dir = root.join("target").join("sbf-build");
    let mut command = Command::new("cargo");
    command
        .current_dir(&root)
        .arg("build-sbf")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--sbf-out-dir")
        .arg(out_dir)
        .env("CARGO_BUILD_JOBS", "3")
        .env("CARGO_TARGET_DIR", &target_dir)
        .stdin(Stdio::null());

    let output = command.output().map_err(|source| RangeError::Tool {
        program: "cargo",
        source,
    })?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{stdout}\n{stderr}");
        return Err(BuildError {
            context: format!(
                "failed to build {slug} ({mode}) with `cargo build-sbf --manifest-path {} --sbf-out-dir {}`",
                manifest.display(),
                out_dir.display()
            ),
            output: tail(&combined, 60),
        }
        .into());
    }

    Ok(())
}

fn tail(text: &str, lines: usize) -> String {
    let collected: Vec<&str> = text.lines().collect();
    if collected.is_empty() {
        return "(no output)".to_owned();
    }
    let start = collected.len().saturating_sub(lines);
    collected[start..].join("\n")
}
