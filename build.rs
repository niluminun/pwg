use std::process::Command;

/// Runs git and returns stdout, or None when git is missing, fails, or this is not a repo.
fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Exports PWG_VERSION as "<major>.<minor>-<hash8>[-dirty]" (see design doc 5.6).
fn main() {
    let hash = git(&["rev-parse", "--short=8", "HEAD"]).unwrap_or_else(|| "unknown".into());
    // Untracked files are ignored so target/ does not make every build dirty.
    let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    let major = std::env::var("CARGO_PKG_VERSION_MAJOR").unwrap_or_default();
    let minor = std::env::var("CARGO_PKG_VERSION_MINOR").unwrap_or_default();
    let suffix = if dirty { "-dirty" } else { "" };
    println!("cargo:rustc-env=PWG_VERSION={major}.{minor}-{hash}{suffix}");
    // Missing paths make cargo rerun every build, which is the right fallback without a repo.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
}
