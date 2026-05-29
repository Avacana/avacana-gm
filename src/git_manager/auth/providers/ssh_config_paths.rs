use super::parser::{unsupported_directive_error, DirectiveSource};
use dirs::home_dir;
use glob::glob;
use std::path::{Path, PathBuf};

pub(super) fn expand_include_paths(
    raw_include: &str,
    base_dir: &Path,
    source: &DirectiveSource,
) -> crate::git_manager::auth::AuthResult<Vec<PathBuf>> {
    let include_path = expand_config_path(raw_include, base_dir);
    let include_pattern = include_path.to_string_lossy();

    if !contains_glob_tokens(raw_include) {
        if include_path.exists() {
            return Ok(vec![include_path]);
        }
        return Ok(Vec::new());
    }

    let mut include_files = Vec::new();
    for entry in glob(&include_pattern).map_err(|error| {
        unsupported_directive_error(
            source,
            format!("invalid Include pattern `{raw_include}`: {error}"),
        )
    })? {
        let include_file = entry.map_err(|error| {
            unsupported_directive_error(
                source,
                format!("failed to resolve Include `{raw_include}`: {error}"),
            )
        })?;
        if include_file.is_file() {
            include_files.push(include_file);
        }
    }
    include_files.sort();
    Ok(include_files)
}

fn contains_glob_tokens(value: &str) -> bool {
    value
        .chars()
        .any(|symbol| matches!(symbol, '*' | '?' | '[' | ']' | '{' | '}'))
}

pub(super) fn expand_config_path(raw_path: &str, base_dir: &Path) -> PathBuf {
    if raw_path == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(raw_path));
    }

    if let Some(rest) = raw_path.strip_prefix("~/") {
        return home_dir().map_or_else(
            || PathBuf::from(raw_path),
            |home| home.join(Path::new(rest)),
        );
    }

    let path = Path::new(raw_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

pub(super) fn normalize_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir()
        .map_or_else(|_| path.to_path_buf(), |current_dir| current_dir.join(path))
}

pub(super) fn default_ssh_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|home| home.join(".ssh").join("config"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        home_dir().map(|home| home.join(".ssh").join("config"))
    }
}
