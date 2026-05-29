use crate::git_manager::auth::{AuthError, AuthErrorCode, AuthResult, NetrcEntry};
use std::fs::File;
use std::io::{BufRead, BufReader, ErrorKind};
use std::path::Path;

#[derive(Debug, Clone)]
struct NetrcToken {
    value: String,
    line: usize,
}

#[derive(Debug, Clone, Default)]
struct NetrcEntryBuilder {
    machine: Option<String>,
    is_default: bool,
    login: Option<String>,
    password: Option<String>,
}

impl NetrcEntryBuilder {
    const fn has_entry_header(&self) -> bool {
        self.is_default || self.machine.is_some()
    }

    fn into_entry(self) -> Option<NetrcEntry> {
        if self.is_default {
            return Some(NetrcEntry::for_default(self.login, self.password));
        }

        self.machine
            .map(|machine| NetrcEntry::for_machine(machine, self.login, self.password))
    }
}

pub(super) fn parse_netrc_file(netrc_path: &Path) -> AuthResult<Vec<NetrcEntry>> {
    let file = match File::open(netrc_path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(Vec::new());
        }
        Err(error) => {
            return Err(netrc_io_error(netrc_path, &error));
        }
    };

    let reader = BufReader::new(file);
    let tokens = tokenize_netrc(reader, netrc_path)?;
    parse_tokens(&tokens, netrc_path)
}

fn tokenize_netrc<R: BufRead>(reader: R, netrc_path: &Path) -> AuthResult<Vec<NetrcToken>> {
    let mut tokens = Vec::new();

    for (line_index, line_result) in reader.lines().enumerate() {
        let line = line_result.map_err(|error| netrc_io_error(netrc_path, &error))?;
        let line_number = line_index + 1;
        let normalized = strip_inline_comment(line.trim());
        if normalized.is_empty() {
            continue;
        }

        for token in normalized.split_whitespace() {
            tokens.push(NetrcToken {
                value: token.to_string(),
                line: line_number,
            });
        }
    }

    Ok(tokens)
}

fn parse_tokens(tokens: &[NetrcToken], netrc_path: &Path) -> AuthResult<Vec<NetrcEntry>> {
    let mut entries = Vec::new();
    let mut builder = NetrcEntryBuilder::default();
    let mut token_index = 0;

    while token_index < tokens.len() {
        let token = &tokens[token_index];
        match token.value.as_str() {
            "machine" => {
                finalize_entry_if_started(&mut entries, &mut builder);
                token_index += 1;
                let machine_token = tokens.get(token_index).ok_or_else(|| {
                    netrc_parse_error(netrc_path, token.line, "missing value after 'machine'")
                })?;
                builder.machine = Some(machine_token.value.clone());
                builder.is_default = false;
            }
            "default" => {
                finalize_entry_if_started(&mut entries, &mut builder);
                builder.is_default = true;
                builder.machine = None;
            }
            "login" => {
                token_index += 1;
                let login_token = tokens.get(token_index).ok_or_else(|| {
                    netrc_parse_error(netrc_path, token.line, "missing value after 'login'")
                })?;
                builder.login = Some(login_token.value.clone());
            }
            "password" => {
                token_index += 1;
                let password_token = tokens.get(token_index).ok_or_else(|| {
                    netrc_parse_error(netrc_path, token.line, "missing value after 'password'")
                })?;
                builder.password = Some(password_token.value.clone());
            }
            "account" => {
                token_index += 1;
                if tokens.get(token_index).is_none() {
                    return Err(netrc_parse_error(
                        netrc_path,
                        token.line,
                        "missing value after 'account'",
                    ));
                }
            }
            "macdef" => {
                return Err(netrc_parse_error(
                    netrc_path,
                    token.line,
                    "macdef is not supported in MVP netrc parser",
                ));
            }
            _ => {}
        }

        token_index += 1;
    }

    finalize_entry_if_started(&mut entries, &mut builder);
    Ok(entries)
}

fn finalize_entry_if_started(entries: &mut Vec<NetrcEntry>, builder: &mut NetrcEntryBuilder) {
    if !builder.has_entry_header() {
        return;
    }

    let completed = std::mem::take(builder);
    if let Some(entry) = completed.into_entry() {
        entries.push(entry);
    }
}

fn strip_inline_comment(line: &str) -> &str {
    if let Some(index) = line.find('#') {
        return line[..index].trim();
    }
    line
}

pub(super) fn netrc_parse_error(
    netrc_path: &Path,
    line: usize,
    details: impl Into<String>,
) -> AuthError {
    let details = details.into();
    let line_suffix = if line > 0 {
        format!(" line {line}")
    } else {
        String::new()
    };

    AuthError::new(
        AuthErrorCode::NoCredentials,
        format!(
            "invalid netrc at {}{}: {details}",
            netrc_path.display(),
            line_suffix
        ),
    )
}

pub(super) fn netrc_io_error(netrc_path: &Path, error: &std::io::Error) -> AuthError {
    AuthError::new(
        AuthErrorCode::NoCredentials,
        format!(
            "failed to read netrc file {}: {error}",
            netrc_path.display()
        ),
    )
}
