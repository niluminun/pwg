use super::args::{Action, Args};
use crate::error::AppError;

fn invalid(msg: impl Into<String>) -> AppError {
    AppError::InvalidArg(msg.into())
}

/// Checks positional count and required flags, then stores the site in `args`.
/// Value bounds (length, symbols, counter) are checked later by the shared rule validation.
pub fn finish(mut args: Args, positionals: Vec<String>) -> Result<Args, AppError> {
    let mut it = positionals.into_iter();
    args.site = it.next();
    if let Some(extra) = it.next() {
        return Err(invalid(format!("unexpected argument '{extra}'")));
    }
    if args.site.is_none() && args.action != Action::List {
        return Err(invalid("missing site"));
    }
    if args.action == Action::Add && args.account.is_none() {
        return Err(invalid("missing '--account'"));
    }
    Ok(args)
}
