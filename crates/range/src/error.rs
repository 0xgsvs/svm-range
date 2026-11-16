use std::{io, path::PathBuf};

#[derive(Debug, thiserror::Error)]
#[error("{context}\n{output}")]
pub struct BuildError {
    pub context: String,
    pub output: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RangeError {
    #[error("unknown challenge `{0}`; run `svm-range list`")]
    UnknownChallenge(String),
    #[error("invalid mode `{0}`; expected `vulnerable` or `patched`")]
    InvalidMode(String),
    #[error("cannot locate the workspace root from {0}")]
    WorkspaceRoot(PathBuf),
    #[error("failed to create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to inspect {path}: {source}")]
    Metadata {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to run `{program}`: {source}")]
    Tool {
        program: &'static str,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Build(#[from] BuildError),
    #[error("LiteSVM: {0}")]
    Svm(String),
    #[error("the build succeeded but no program artifact exists at {0}")]
    MissingArtifact(PathBuf),
    #[error("challenge `{slug}` is broken: {detail}")]
    Broken { slug: &'static str, detail: String },
    #[error("challenge `{0}` already exists at {1}")]
    AlreadyExists(String, PathBuf),
    #[error("invalid challenge slug `{0}`")]
    InvalidSlug(String),
}
