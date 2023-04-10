//! IO utilities.

use std::path::{Path, PathBuf};

use crate::error::{with_err_context, ErrorContext, Result};

pub fn create_dir_all(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    with_err_context(std::fs::create_dir_all(path), || {
        ErrorContext::CreateDir(path.to_path_buf())
    })?;
    Ok(())
}

pub fn create_file(path: impl AsRef<Path>) -> Result<std::fs::File> {
    let path = path.as_ref();
    let file = with_err_context(std::fs::File::create(path), || {
        ErrorContext::CreateFile(path.to_path_buf())
    })?;
    Ok(file)
}

pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();
    let data = with_err_context(std::fs::read_to_string(path), || {
        ErrorContext::ReadFile(path.to_path_buf())
    })?;
    Ok(data)
}

pub fn canonicalize<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path = path.as_ref();
    let path = with_err_context(std::fs::canonicalize(path), || {
        ErrorContext::ReadFile(path.to_path_buf())
    })?;
    Ok(path)
}
