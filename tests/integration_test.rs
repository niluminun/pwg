use std::path::{Path, PathBuf};

use pwg::cli::args::{parse, Parsed};
use pwg::error::AppError;
use pwg::external::serde_wrapper::{from_str, Deserialize};
use pwg::{execute, run};

#[derive(Deserialize)]
struct Golden {
    key: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    args: Vec<String>,
    password: String,
}

const KEY: &str = "pwg golden test key";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pwg-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Runs one CLI call through parse + execute, pointing at the given profile file.
fn call(argv: &[&str], profile: &Path, key: Option<&str>) -> Result<String, AppError> {
    let mut full: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
    full.push("--profile".into());
    full.push(profile.display().to_string());
    match parse(full)? {
        Parsed::Run(args) => execute(&args, key.map(str::as_bytes)),
        other => panic!("expected Run, got {other:?}"),
    }
}

#[test]
fn golden_vectors() {
    let golden: Golden = from_str(include_str!("fixtures/golden.json")).unwrap();
    let profile = scratch("golden").join("none.json");
    assert!(golden.cases.len() >= 6);
    for case in &golden.cases {
        let argv: Vec<&str> = case.args.iter().map(String::as_str).collect();
        let got = call(&argv, &profile, Some(&golden.key)).unwrap();
        assert_eq!(got, case.password, "case '{}'", case.name);
    }
}

#[test]
fn profile_round_trip() {
    let dir = scratch("roundtrip");
    let file = dir.join("sub").join("profiles.json");

    // gen and list on a missing file: empty profiles, nothing printed.
    assert_eq!(call(&["list"], &file, None).unwrap(), "");

    let added = call(
        &["add", "GitHub.com", "--account", "Budi@Gmail.com", "--length", "16", "--require", "l1,u1,d1"],
        &file,
        None,
    );
    assert_eq!(added.unwrap(), "Added github.com (budi@gmail.com).");

    assert_eq!(
        call(&["list"], &file, None).unwrap(),
        "SITE         ACCOUNT          LEN   REQUIRE    COUNTER\n\
         github.com   budi@gmail.com   16    l1,u1,d1   1"
    );

    // gen uses the stored rules, same as passing them as flags.
    let stored = call(&["gen", "github.com"], &file, Some(KEY)).unwrap();
    let explicit = call(
        &["gen", "github.com", "--account", "budi@gmail.com", "--length", "16", "--require", "l1,u1,d1"],
        &file,
        Some(KEY),
    )
    .unwrap();
    assert_eq!(stored, explicit);
    assert_eq!(stored.len(), 16);

    // bump moves to counter 2; --counter 1 still gives the old password.
    assert_eq!(
        call(&["bump", "github.com"], &file, None).unwrap(),
        "github.com (budi@gmail.com): counter is now 2."
    );
    let bumped = call(&["gen", "github.com"], &file, Some(KEY)).unwrap();
    assert_ne!(bumped, stored);
    assert_eq!(call(&["gen", "github.com", "--counter", "1"], &file, Some(KEY)).unwrap(), stored);

    // gen --counter never writes anything.
    assert!(call(&["list", "github.com"], &file, None).unwrap().ends_with("2"));
}

#[cfg(unix)]
#[test]
fn private_modes() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("modes");
    let file = dir.join("sub").join("profiles.json");
    call(&["add", "a.com", "--account", "x"], &file, None).unwrap();
    let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&file), 0o600);
    assert_eq!(mode(file.parent().unwrap()), 0o700);
    assert!(!file.with_file_name("profiles.json.tmp").exists());
}

#[test]
fn error_cases() {
    let dir = scratch("errors");
    let file = dir.join("profiles.json");

    // No action: the default action is not allowed.
    assert_eq!(parse(["github.com"]), Err(AppError::AmbiguousAction));

    call(&["add", "a.com", "--account", "x"], &file, None).unwrap();
    call(&["add", "a.com", "--account", "y"], &file, None).unwrap();

    assert_eq!(
        call(&["add", "A.com", "--account", "X"], &file, None),
        Err(AppError::DuplicateProfile { site: "a.com".into(), account: "x".into() })
    );
    assert_eq!(
        call(&["gen", "a.com"], &file, Some(KEY)),
        Err(AppError::AmbiguousAccount { site: "a.com".into() })
    );
    assert_eq!(
        call(&["bump", "a.com"], &file, None),
        Err(AppError::AmbiguousAccount { site: "a.com".into() })
    );
    assert_eq!(
        call(&["bump", "nope.com"], &file, None),
        Err(AppError::NoProfile { site: "nope.com".into() })
    );
    assert_eq!(
        call(&["gen", "nope.com"], &file, Some(KEY)),
        Err(AppError::InvalidArg("missing '--account'".into()))
    );
    // Missing profile file: bump is an error, not an empty success.
    let missing = dir.join("missing.json");
    assert_eq!(
        call(&["bump", "a.com"], &missing, None),
        Err(AppError::NoProfile { site: "a.com".into() })
    );
    // Rules validation applies to flags.
    let e = call(&["gen", "a.com", "--account", "x", "--length", "0"], &file, Some(KEY)).unwrap_err();
    assert_eq!(e.to_string(), "error: length must be between 1 and 128");
    let e = call(&["gen", "a.com", "--account", "x", "--counter", "0"], &file, Some(KEY)).unwrap_err();
    assert_eq!(e.to_string(), "error: counter must be at least 1");
    assert!(matches!(
        call(&["gen", "a.com", "--account", "x"], &file, None),
        Err(AppError::KeyRead(_))
    ));
}

#[test]
fn bad_profile_files() {
    let dir = scratch("badfiles");
    std::fs::create_dir_all(&dir).unwrap();
    let cases = [
        ("broken.json", "{ not json"),
        ("unknown_field.json", r#"{"version":1,"profiles":[{"site":"a.com","account":"x","lenght":16,"counter":1}]}"#),
        ("unknown_top.json", r#"{"version":1,"profiles":[],"extra":1}"#),
        ("version.json", r#"{"version":2,"profiles":[]}"#),
        ("no_require.json", r#"{"version":1,"profiles":[{"site":"a.com","account":"x","length":16,"counter":1}]}"#),
        ("not_normalized.json", r#"{"version":1,"profiles":[{"site":"A.com","account":"x","length":16,"require":{"lower":1},"counter":1}]}"#),
        ("counter0.json", r#"{"version":1,"profiles":[{"site":"a.com","account":"x","length":16,"require":{"lower":1},"counter":0}]}"#),
        ("duplicate.json", r#"{"version":1,"profiles":[{"site":"a.com","account":"x","length":16,"require":{"lower":1},"counter":1},{"site":"a.com","account":"x","length":16,"require":{"lower":1},"counter":1}]}"#),
    ];
    for (name, body) in cases {
        let file = dir.join(name);
        std::fs::write(&file, body).unwrap();
        let e = call(&["list"], &file, None).unwrap_err();
        assert!(matches!(e, AppError::BadProfile { .. }), "{name}: {e}");
    }
    // A valid hand-written file without symbols falls back to the default symbols.
    let ok = dir.join("ok.json");
    std::fs::write(&ok, r#"{"version":1,"profiles":[{"site":"a.com","account":"x","length":16,"require":{"lower":1,"symbol":1},"counter":3}]}"#).unwrap();
    assert!(call(&["list"], &ok, None).unwrap().contains("l1,s1"));
}

#[test]
fn help_and_version_through_run() {
    let out = run(parse(["--version"]).unwrap()).unwrap();
    assert_eq!(out, format!("pwg {}", env!("PWG_VERSION")));
    // Shape: "pwg 0.1-<hash or unknown>[-dirty]".
    assert!(out.starts_with("pwg 0.1-"), "{out}");
    let tool = run(parse(["-h"]).unwrap()).unwrap();
    assert!(tool.starts_with("pwg - derives a site password"));
    let gen = run(parse(["gen", "-h"]).unwrap()).unwrap();
    assert!(gen.starts_with("gen - derives and prints"));
}
