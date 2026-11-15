pub mod cli;
pub mod derive;
pub mod error;
pub mod external;
pub mod generate;
pub mod profile;

use std::path::Path;

use cli::args::{Action, Args, Parsed};
use cli::help;
use derive::normalize::{normalize_account, normalize_site};
use derive::{ARGON_M_KIB, ARGON_P, ARGON_T};
use error::AppError;
use external::rpassword_wrapper::{prompt_key, Key};
use profile::rules::{validate_counter, Profile, Rules};
use profile::store;

/// Entry point for main: help, version, or a real action. Reads the key only for `gen`.
pub fn run(parsed: Parsed) -> Result<String, AppError> {
    match parsed {
        Parsed::Help(action) => Ok(help::text(action).to_string()),
        Parsed::Version => Ok(format!("pwg {}", env!("PWG_VERSION"))),
        Parsed::Run(args) => {
            let key = if args.action == Action::Gen {
                Some(read_key(args.confirm)?)
            } else {
                None
            };
            execute(&args, key.as_ref().map(|k| k.as_bytes()))
        }
    }
}

/// All the work, returning the stdout text. Tests call this directly because a TTY prompt
/// cannot be tested.
pub fn execute(args: &Args, key: Option<&[u8]>) -> Result<String, AppError> {
    let path = store::resolve_path(args.profile.as_deref())?;
    if args.verbose {
        eprintln!("profile: {}", path.display());
    }
    match args.action {
        Action::Gen => cmd_gen(args, key, &path),
        Action::Add => cmd_add(args, &path),
        Action::Bump => cmd_bump(args, &path),
        Action::List => cmd_list(args, &path),
    }
}

fn read_key(confirm: bool) -> Result<Key, AppError> {
    let key = prompt_key("Key: ").map_err(AppError::KeyRead)?;
    if key.is_empty() {
        return Err(AppError::KeyRead("key is empty".into()));
    }
    if confirm {
        let again = prompt_key("Confirm key: ").map_err(AppError::KeyRead)?;
        if *key != *again {
            return Err(AppError::KeyMismatch);
        }
    }
    Ok(key)
}

fn site_of(args: &Args) -> Result<String, AppError> {
    normalize_site(args.site.as_deref().unwrap_or_default()).map_err(AppError::InvalidArg)
}

fn account_flag(args: &Args) -> Result<Option<String>, AppError> {
    args.account
        .as_deref()
        .map(normalize_account)
        .transpose()
        .map_err(AppError::InvalidArg)
}

/// Flag values win over `base` (a stored profile or the defaults). `--require` replaces whole.
fn merge_rules(args: &Args, base: Rules) -> Rules {
    Rules {
        length: args.length.unwrap_or(base.length),
        require: args.require.unwrap_or(base.require),
        symbols: args.symbols.clone().unwrap_or(base.symbols),
    }
}

fn check(rules: &Rules, counter: u32) -> Result<(), AppError> {
    rules.validate().map_err(AppError::InvalidArg)?;
    validate_counter(counter).map_err(AppError::InvalidArg)
}

fn cmd_gen(args: &Args, key: Option<&[u8]>, path: &Path) -> Result<String, AppError> {
    let site = site_of(args)?;
    let flag_account = account_flag(args)?;
    let profiles = store::load(path)?;
    let found = store::find(&profiles, &site, flag_account.as_deref())?.map(|i| &profiles[i]);
    let account = match (flag_account, found) {
        (Some(a), _) => a,
        (None, Some(p)) => p.account.clone(),
        (None, None) => return Err(AppError::InvalidArg("missing '--account'".into())),
    };
    let rules = merge_rules(args, found.map(Profile::rules).unwrap_or_default());
    let counter = args.counter.or(found.map(|p| p.counter)).unwrap_or(1);
    check(&rules, counter)?;
    if args.verbose {
        eprintln!(
            "rules: length={} require={} symbols={} counter={counter}",
            rules.length,
            rules.require.to_spec(),
            rules.symbols
        );
        eprintln!("kdf: argon2id v=19 m={ARGON_M_KIB} t={ARGON_T} p={ARGON_P} out=32");
    }
    let key = key.ok_or_else(|| AppError::KeyRead("no key given".into()))?;
    let mut stream = derive::derive_stream(key, &site, &account, counter)?;
    Ok(generate::generate(&mut stream, &rules))
}

fn cmd_add(args: &Args, path: &Path) -> Result<String, AppError> {
    let site = site_of(args)?;
    let account = account_flag(args)?
        .ok_or_else(|| AppError::InvalidArg("missing '--account'".into()))?;
    let rules = merge_rules(args, Rules::default());
    let counter = args.counter.unwrap_or(1);
    check(&rules, counter)?;
    let mut profiles = store::load(path)?;
    if store::find(&profiles, &site, Some(&account))?.is_some() {
        return Err(AppError::DuplicateProfile { site, account });
    }
    profiles.push(Profile {
        site: site.clone(),
        account: account.clone(),
        length: rules.length,
        require: rules.require,
        symbols: rules.symbols,
        counter,
    });
    store::save(path, &profiles)?;
    Ok(format!("Added {site} ({account})."))
}

fn cmd_bump(args: &Args, path: &Path) -> Result<String, AppError> {
    let site = site_of(args)?;
    let mut profiles = store::load(path)?;
    let idx = store::find(&profiles, &site, account_flag(args)?.as_deref())?
        .ok_or_else(|| AppError::NoProfile { site: site.clone() })?;
    let p = &mut profiles[idx];
    p.counter = p
        .counter
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidArg("counter cannot go higher".into()))?;
    let line = format!("{} ({}): counter is now {}.", p.site, p.account, p.counter);
    store::save(path, &profiles)?;
    Ok(line)
}

fn cmd_list(args: &Args, path: &Path) -> Result<String, AppError> {
    let site = args
        .site
        .as_deref()
        .map(normalize_site)
        .transpose()
        .map_err(AppError::InvalidArg)?;
    let account = account_flag(args)?;
    let profiles = store::load(path)?;
    let shown: Vec<&Profile> = profiles
        .iter()
        .filter(|p| site.as_ref().is_none_or(|s| p.site == *s))
        .filter(|p| account.as_ref().is_none_or(|a| p.account == *a))
        .collect();
    Ok(store::render_list(&shown))
}
