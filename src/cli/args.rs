use std::ffi::OsString;
use std::path::PathBuf;
use std::str::FromStr;

use super::validate;
use crate::error::AppError;
use crate::profile::rules::{Require, MAX_LENGTH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Gen,
    Add,
    Bump,
    List,
}

impl Action {
    pub fn name(self) -> &'static str {
        match self {
            Action::Gen => "gen",
            Action::Add => "add",
            Action::Bump => "bump",
            Action::List => "list",
        }
    }

    fn from_token(t: &str) -> Option<Action> {
        match t {
            "gen" | "-g" => Some(Action::Gen),
            "add" | "-a" => Some(Action::Add),
            "bump" | "-b" => Some(Action::Bump),
            "list" | "-l" => Some(Action::List),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Args {
    pub action: Action,
    pub site: Option<String>,
    pub account: Option<String>,
    pub length: Option<usize>,
    pub require: Option<Require>,
    pub symbols: Option<String>,
    pub counter: Option<u32>,
    pub profile: Option<PathBuf>,
    pub confirm: bool,
    pub verbose: bool,
}

#[derive(Debug, PartialEq)]
pub enum Parsed {
    Help(Option<Action>),
    Version,
    Run(Args),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flag {
    Account,
    Length,
    Require,
    Symbols,
    Counter,
    Profile,
    Confirm,
    Verbose,
    Help,
}

struct FlagDef {
    flag: Flag,
    long: &'static str,
    short: Option<&'static str>,
    takes_value: bool,
    actions: &'static [Action],
}

const EVERY: &[Action] = &[Action::Gen, Action::Add, Action::Bump, Action::List];
const GEN_ADD: &[Action] = &[Action::Gen, Action::Add];
const GEN_ONLY: &[Action] = &[Action::Gen];

// Single source of truth for flags: tokenizing, help scan and validation all read this table.
const FLAGS: &[FlagDef] = &[
    FlagDef { flag: Flag::Account, long: "--account", short: None, takes_value: true, actions: EVERY },
    FlagDef { flag: Flag::Length, long: "--length", short: None, takes_value: true, actions: GEN_ADD },
    FlagDef { flag: Flag::Require, long: "--require", short: None, takes_value: true, actions: GEN_ADD },
    FlagDef { flag: Flag::Symbols, long: "--symbols", short: None, takes_value: true, actions: GEN_ADD },
    FlagDef { flag: Flag::Counter, long: "--counter", short: None, takes_value: true, actions: GEN_ADD },
    FlagDef { flag: Flag::Profile, long: "--profile", short: None, takes_value: true, actions: EVERY },
    FlagDef { flag: Flag::Confirm, long: "--confirm", short: None, takes_value: false, actions: GEN_ONLY },
    FlagDef { flag: Flag::Verbose, long: "--verbose", short: Some("-v"), takes_value: false, actions: EVERY },
    FlagDef { flag: Flag::Help, long: "--help", short: Some("-h"), takes_value: false, actions: EVERY },
];

fn invalid(msg: String) -> AppError {
    AppError::InvalidArg(msg)
}

/// Splits "--name=value" into name and inline value; short flags never carry one.
fn split_flag(token: &str) -> (&str, Option<&str>) {
    match token.split_once('=') {
        Some((n, v)) if n.starts_with("--") => (n, Some(v)),
        _ => (token, None),
    }
}

fn lookup(name: &str, action: Action) -> Option<&'static FlagDef> {
    FLAGS
        .iter()
        .find(|d| (d.long == name || d.short == Some(name)) && d.actions.contains(&action))
}

/// Parses argv (without the program name) into a help request, a version request, or a run.
pub fn parse<I, S>(argv: I) -> Result<Parsed, AppError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut tokens = Vec::new();
    for a in argv {
        let s = a
            .into()
            .into_string()
            .map_err(|_| invalid("argument is not valid UTF-8".into()))?;
        tokens.push(s);
    }
    let Some(first) = tokens.first() else {
        return Err(AppError::AmbiguousAction);
    };
    match first.as_str() {
        "-h" | "--help" => return Ok(Parsed::Help(None)),
        "--version" => return Ok(Parsed::Version),
        _ => {}
    }
    let action = Action::from_token(first).ok_or(AppError::AmbiguousAction)?;
    let rest = &tokens[1..];
    if wants_help(action, rest) {
        return Ok(Parsed::Help(Some(action)));
    }
    tokenize(action, rest).map(Parsed::Run)
}

/// True when -h/--help appears as a flag. A value flag swallows the next token, so
/// `--symbols -h` is a value, not a help request.
fn wants_help(action: Action, rest: &[String]) -> bool {
    let mut i = 0;
    while i < rest.len() {
        let (name, inline) = split_flag(&rest[i]);
        if name == "-h" || name == "--help" {
            return true;
        }
        if let Some(def) = lookup(name, action) {
            if def.takes_value && inline.is_none() {
                i += 1;
            }
        }
        i += 1;
    }
    false
}

fn tokenize(action: Action, rest: &[String]) -> Result<Args, AppError> {
    let mut args = Args {
        action,
        site: None,
        account: None,
        length: None,
        require: None,
        symbols: None,
        counter: None,
        profile: None,
        confirm: false,
        verbose: false,
    };
    let mut positionals = Vec::new();
    let mut seen: Vec<Flag> = Vec::new();
    let mut i = 0;
    while i < rest.len() {
        let token = &rest[i];
        i += 1;
        if !token.starts_with('-') || token == "-" {
            positionals.push(token.clone());
            continue;
        }
        let (name, inline) = split_flag(token);
        let Some(def) = lookup(name, action) else {
            return Err(invalid(format!("unknown flag '{name}' for '{}'", action.name())));
        };
        if seen.contains(&def.flag) {
            return Err(invalid(format!("flag '{}' given twice", def.long)));
        }
        seen.push(def.flag);
        let value = match (def.takes_value, inline) {
            (false, Some(_)) => {
                return Err(invalid(format!("flag '{}' takes no value", def.long)))
            }
            (false, None) => String::new(),
            (true, Some(v)) => v.to_string(),
            (true, None) => {
                let v = rest
                    .get(i)
                    .ok_or_else(|| invalid(format!("flag '{}' needs a value", def.long)))?;
                i += 1;
                v.clone()
            }
        };
        apply(&mut args, def, value)?;
    }
    validate::finish(args, positionals)
}

fn apply(args: &mut Args, def: &FlagDef, value: String) -> Result<(), AppError> {
    match def.flag {
        Flag::Account => args.account = Some(value),
        Flag::Length => args.length = Some(number(&value, def.long)?),
        Flag::Require => {
            let spec = parse_require(&value)
                .ok_or_else(|| invalid(format!("invalid value '{value}' for '--require'")))?;
            args.require = Some(spec);
        }
        Flag::Symbols => args.symbols = Some(value),
        Flag::Counter => args.counter = Some(number(&value, def.long)?),
        Flag::Profile => args.profile = Some(PathBuf::from(value)),
        Flag::Confirm => args.confirm = true,
        Flag::Verbose => args.verbose = true,
        Flag::Help => {}
    }
    Ok(())
}

fn number<T: FromStr>(value: &str, flag: &str) -> Result<T, AppError> {
    value
        .parse()
        .map_err(|_| invalid(format!("invalid number '{value}' for '{flag}'")))
}

/// Parses "l1,u1,d2,s1": each class letter at most once, count 0..=128. Unmentioned classes are 0.
fn parse_require(spec: &str) -> Option<Require> {
    let mut counts = [0usize; 4];
    let mut seen = [false; 4];
    for part in spec.split(',') {
        let mut chars = part.chars();
        let idx = match chars.next()? {
            'l' => 0,
            'u' => 1,
            'd' => 2,
            's' => 3,
            _ => return None,
        };
        let digits = chars.as_str();
        if seen[idx] || digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        seen[idx] = true;
        counts[idx] = digits.parse().ok().filter(|n| *n <= MAX_LENGTH)?;
    }
    Some(Require::from_counts(counts))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(v: &[&str]) -> Result<Parsed, AppError> {
        parse(v.iter().copied())
    }

    fn args(v: &[&str]) -> Args {
        match parse_strs(v).unwrap() {
            Parsed::Run(a) => a,
            other => panic!("expected Run, got {other:?}"),
        }
    }

    fn err(v: &[&str]) -> String {
        parse_strs(v).unwrap_err().to_string()
    }

    #[test]
    fn value_forms() {
        assert_eq!(args(&["gen", "x", "--length", "16"]), args(&["gen", "x", "--length=16"]));
        assert_eq!(args(&["gen", "x", "--length=16"]).length, Some(16));
    }

    #[test]
    fn value_may_start_with_dash() {
        assert_eq!(args(&["gen", "x", "--symbols", "-_"]).symbols.as_deref(), Some("-_"));
        assert_eq!(args(&["gen", "x", "--symbols=-_"]).symbols.as_deref(), Some("-_"));
    }

    #[test]
    fn flags_before_and_after_site() {
        let a = args(&["gen", "--account", "a", "x", "-v", "--confirm"]);
        assert_eq!(a.site.as_deref(), Some("x"));
        assert_eq!(a.account.as_deref(), Some("a"));
        assert!(a.verbose && a.confirm);
    }

    #[test]
    fn short_and_long_actions() {
        for (short, long) in [("-g", "gen"), ("-a", "add"), ("-b", "bump"), ("-l", "list")] {
            let rest: &[&str] = &["x", "--account", "a"];
            let s: Vec<&str> = std::iter::once(short).chain(rest.iter().copied()).collect();
            let l: Vec<&str> = std::iter::once(long).chain(rest.iter().copied()).collect();
            assert_eq!(args(&s), args(&l));
        }
    }

    #[test]
    fn duplicate_flag() {
        assert_eq!(err(&["gen", "x", "--length", "1", "--length", "2"]), "error: flag '--length' given twice");
        assert_eq!(err(&["gen", "x", "-v", "--verbose"]), "error: flag '--verbose' given twice");
    }

    #[test]
    fn unknown_and_misplaced_flags() {
        assert_eq!(err(&["gen", "x", "--foo"]), "error: unknown flag '--foo' for 'gen'");
        assert_eq!(err(&["bump", "x", "--length", "3"]), "error: unknown flag '--length' for 'bump'");
        assert_eq!(err(&["gen", "x", "-hv"]), "error: unknown flag '-hv' for 'gen'");
        assert_eq!(err(&["list", "--confirm"]), "error: unknown flag '--confirm' for 'list'");
    }

    #[test]
    fn value_errors() {
        assert_eq!(err(&["gen", "x", "--length"]), "error: flag '--length' needs a value");
        assert_eq!(err(&["gen", "x", "--confirm=1"]), "error: flag '--confirm' takes no value");
        assert_eq!(err(&["gen", "x", "--length", "abc"]), "error: invalid number 'abc' for '--length'");
        assert_eq!(err(&["gen", "x", "--counter", "-1"]), "error: invalid number '-1' for '--counter'");
    }

    #[test]
    fn positional_errors() {
        assert_eq!(err(&["gen", "a", "b"]), "error: unexpected argument 'b'");
        assert_eq!(err(&["gen"]), "error: missing site");
        assert_eq!(err(&["add", "x"]), "error: missing '--account'");
        assert!(args(&["list"]).site.is_none());
    }

    #[test]
    fn action_is_required_first() {
        assert_eq!(err(&[]), "error: ambiguous, which action?");
        assert_eq!(err(&["github.com"]), "error: ambiguous, which action?");
        assert_eq!(err(&["-v", "gen", "x"]), "error: ambiguous, which action?");
        assert_eq!(err(&["--bogus"]), "error: ambiguous, which action?");
    }

    #[test]
    fn help_and_version() {
        assert_eq!(parse_strs(&["-h"]), Ok(Parsed::Help(None)));
        assert_eq!(parse_strs(&["--help"]), Ok(Parsed::Help(None)));
        assert_eq!(parse_strs(&["--version"]), Ok(Parsed::Version));
        assert_eq!(parse_strs(&["gen", "--bogus", "-h"]), Ok(Parsed::Help(Some(Action::Gen))));
        assert_eq!(parse_strs(&["-b", "--help"]), Ok(Parsed::Help(Some(Action::Bump))));
        // "-h" right after a value flag is its value, not a help request.
        assert_eq!(err(&["gen", "--symbols", "-h"]), "error: missing site");
    }

    #[test]
    fn require_spec() {
        assert_eq!(parse_require("l1,u1,d2,s1"), Some(Require::from_counts([1, 1, 2, 1])));
        assert_eq!(parse_require("d3"), Some(Require::from_counts([0, 0, 3, 0])));
        assert_eq!(parse_require("s0,l128"), Some(Require::from_counts([128, 0, 0, 0])));
        for bad in ["", "l", "l1,l2", "x1", "l129", "l-1", "l1,", "1l", "l+1"] {
            assert_eq!(parse_require(bad), None, "{bad:?}");
        }
        assert_eq!(err(&["gen", "x", "--require", "l1,l2"]), "error: invalid value 'l1,l2' for '--require'");
    }
}
