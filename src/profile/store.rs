use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::rules::{validate_counter, Profile};
use crate::derive::normalize::{normalize_account, normalize_site};
use crate::error::AppError;
use crate::external::serde_wrapper::{self, Deserialize, Serialize};

const FILE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileFile {
    version: u32,
    profiles: Vec<Profile>,
}

fn show(path: &Path) -> String {
    path.display().to_string()
}

/// Explicit path first, then the per-OS default (spec 4.5).
pub fn resolve_path(explicit: Option<&Path>) -> Result<PathBuf, AppError> {
    if let Some(p) = explicit {
        return Ok(p.to_path_buf());
    }
    let env = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    let base = if cfg!(windows) {
        env("APPDATA").map(PathBuf::from)
    } else {
        env("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env("HOME").map(|h| PathBuf::from(h).join(".config")))
    };
    match base {
        Some(b) => Ok(b.join("pwg").join("profiles.json")),
        None => Err(AppError::InvalidArg("cannot determine profile path".into())),
    }
}

/// Reads and validates the file. A missing file counts as empty.
pub fn load(path: &Path) -> Result<Vec<Profile>, AppError> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(AppError::CannotOpen { path: show(path), reason: e.to_string() })
        }
    };
    let bad = |reason: String| AppError::BadProfile { path: show(path), reason };
    let file: ProfileFile = serde_wrapper::from_str(&text).map_err(bad)?;
    if file.version != FILE_VERSION {
        return Err(bad(format!("unsupported version {}", file.version)));
    }
    for (i, p) in file.profiles.iter().enumerate() {
        check_profile(p).map_err(|r| bad(format!("profile {}: {r}", i + 1)))?;
        if file.profiles[..i].iter().any(|q| q.site == p.site && q.account == p.account) {
            return Err(bad(format!("profile {}: duplicate site and account", i + 1)));
        }
    }
    Ok(file.profiles)
}

fn check_profile(p: &Profile) -> Result<(), String> {
    if normalize_site(&p.site)? != p.site {
        return Err("site is not normalized".into());
    }
    if normalize_account(&p.account)? != p.account {
        return Err("account is not normalized".into());
    }
    p.rules().validate()?;
    validate_counter(p.counter)
}

/// Index of the matching profile. Without an account, more than one match is an error (spec 4.4).
pub fn find(profiles: &[Profile], site: &str, account: Option<&str>) -> Result<Option<usize>, AppError> {
    let mut hits = profiles
        .iter()
        .enumerate()
        .filter(|(_, p)| p.site == site && account.is_none_or(|a| p.account == a))
        .map(|(i, _)| i);
    let first = hits.next();
    if account.is_none() && hits.next().is_some() {
        return Err(AppError::AmbiguousAccount { site: site.to_string() });
    }
    Ok(first)
}

/// Writes to `<file>.tmp`, flushes, then renames over the real file.
pub fn save(path: &Path, profiles: &[Profile]) -> Result<(), AppError> {
    let write_err = |reason: String| AppError::CannotWrite { path: show(path), reason };
    let file = ProfileFile { version: FILE_VERSION, profiles: profiles.to_vec() };
    let mut text = serde_wrapper::to_string_pretty(&file).map_err(write_err)?;
    text.push('\n');
    write_atomic(path, &text).map_err(|e| AppError::CannotWrite {
        path: show(path),
        reason: e.to_string(),
    })
}

fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(dir)?;
    }
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    let tmp = path.with_file_name(name);
    // A stale temp file could carry looser permissions, so start from nothing.
    let _ = fs::remove_file(&tmp);
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
    let mut f = opts.open(&tmp)?;
    f.write_all(text.as_bytes())?;
    f.flush()?;
    f.sync_all()?;
    drop(f);
    fs::rename(&tmp, path)
}

/// Flat table sorted by site then account; empty input prints nothing.
pub fn render_list(profiles: &[&Profile]) -> String {
    if profiles.is_empty() {
        return String::new();
    }
    let mut rows: Vec<[String; 5]> = profiles
        .iter()
        .map(|p| {
            [p.site.clone(), p.account.clone(), p.length.to_string(), p.require.to_spec(), p.counter.to_string()]
        })
        .collect();
    rows.sort();
    rows.insert(0, ["SITE", "ACCOUNT", "LEN", "REQUIRE", "COUNTER"].map(String::from));
    let widths: Vec<usize> = (0..5).map(|c| rows.iter().map(|r| r[c].len()).max().unwrap_or(0)).collect();
    let lines: Vec<String> = rows
        .iter()
        .map(|r| {
            let cells: Vec<String> = (0..5)
                .map(|c| if c == 4 { r[c].clone() } else { format!("{:<w$}", r[c], w = widths[c] + 3) })
                .collect();
            cells.concat()
        })
        .collect();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::rules::Require;

    fn p(site: &str, account: &str) -> Profile {
        Profile {
            site: site.into(),
            account: account.into(),
            length: 16,
            require: Require::from_counts([1, 1, 1, 0]),
            symbols: "!@".into(),
            counter: 1,
        }
    }

    #[test]
    fn find_rules() {
        let v = vec![p("a.com", "x"), p("a.com", "y"), p("b.com", "z")];
        assert_eq!(find(&v, "a.com", Some("y")).unwrap(), Some(1));
        assert_eq!(find(&v, "b.com", None).unwrap(), Some(2));
        assert_eq!(find(&v, "c.com", None).unwrap(), None);
        assert_eq!(find(&v, "a.com", Some("q")).unwrap(), None);
        assert_eq!(
            find(&v, "a.com", None),
            Err(AppError::AmbiguousAccount { site: "a.com".into() })
        );
    }

    #[test]
    fn list_layout() {
        let a = p("github.com", "budi@gmail.com");
        assert_eq!(
            render_list(&[&a]),
            "SITE         ACCOUNT          LEN   REQUIRE    COUNTER\n\
             github.com   budi@gmail.com   16    l1,u1,d1   1"
        );
        assert_eq!(render_list(&[]), "");
    }
}
