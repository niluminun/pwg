use super::args::Action;

const TOOL: &str = "\
pwg - derives a site password from a private key and a saved rule profile

Usage: pwg <action> [site] [flags]

Actions:
  -g, gen    Derive and print a password
  -a, add    Save a profile for a site and account
  -b, bump   Raise the counter of a profile by one
  -l, list   List saved profiles

Flags:
  -v, --verbose   Print profile path, rules and KDF parameters to stderr
  --version       Print the version

Run 'pwg <action> -h' for flags specific to that action.";

const GEN: &str = "\
gen - derives and prints the password for a site and account

Usage: pwg gen <site> [flags]

Flags:
  --account <a>      Account name or email (required when no profile matches)
  --length <n>       Password length (default: profile value, else 20)
  --require <spec>   Minimum characters per class, e.g. l1,u1,d2,s1
  --symbols <s>      Allowed symbol characters (default: !@#$%^&*-_)
  --counter <n>      Counter for this call only (default: profile value, else 1)
  --profile <path>   Profile file to read
  --confirm          Ask for the key twice
  -v, --verbose      Print profile path, rules and KDF parameters to stderr
  -h, --help         Show this help";

const ADD: &str = "\
add - saves a profile for a site and account

Usage: pwg add <site> --account <a> [flags]

Flags:
  --account <a>      Account name or email (required)
  --length <n>       Password length (default: 20)
  --require <spec>   Minimum characters per class (default: l1,u1,d1,s1)
  --symbols <s>      Allowed symbol characters (default: !@#$%^&*-_)
  --counter <n>      Starting counter (default: 1)
  --profile <path>   Profile file to write
  -v, --verbose      Print profile path to stderr
  -h, --help         Show this help";

const BUMP: &str = "\
bump - raises the counter of a saved profile by one

Usage: pwg bump <site> [flags]

Flags:
  --account <a>      Account name or email (needed when the site has several)
  --profile <path>   Profile file to update
  -v, --verbose      Print profile path to stderr
  -h, --help         Show this help";

const LIST: &str = "\
list - lists saved profiles

Usage: pwg list [site] [flags]

Flags:
  --account <a>      Show only this account
  --profile <path>   Profile file to read
  -v, --verbose      Print profile path to stderr
  -h, --help         Show this help";

pub fn text(action: Option<Action>) -> &'static str {
    match action {
        None => TOOL,
        Some(Action::Gen) => GEN,
        Some(Action::Add) => ADD,
        Some(Action::Bump) => BUMP,
        Some(Action::List) => LIST,
    }
}
