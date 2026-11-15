use crate::external::serde_wrapper::{Deserialize, Serialize};

pub const MAX_LENGTH: usize = 128;
pub const MAX_SYMBOLS: usize = 32;
pub const DEFAULT_LENGTH: usize = 20;
pub const DEFAULT_SYMBOLS: &str = "!@#$%^&*-_";

/// Minimum characters per class. A missing field in the profile file means 0.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Require {
    pub lower: usize,
    pub upper: usize,
    pub digit: usize,
    pub symbol: usize,
}

impl Require {
    pub const DEFAULT: Require = Require { lower: 1, upper: 1, digit: 1, symbol: 1 };
    const LETTERS: [char; 4] = ['l', 'u', 'd', 's'];

    pub fn from_counts(c: [usize; 4]) -> Self {
        Require { lower: c[0], upper: c[1], digit: c[2], symbol: c[3] }
    }

    /// Counts in the fixed class order: lower, upper, digit, symbol.
    pub fn counts(&self) -> [usize; 4] {
        [self.lower, self.upper, self.digit, self.symbol]
    }

    pub fn total(&self) -> usize {
        self.counts().iter().fold(0, |a, &b| a.saturating_add(b))
    }

    /// Renders non-zero classes as "l1,u1,d2".
    pub fn to_spec(&self) -> String {
        let parts: Vec<String> = Self::LETTERS
            .iter()
            .zip(self.counts())
            .filter(|(_, n)| *n > 0)
            .map(|(l, n)| format!("{l}{n}"))
            .collect();
        parts.join(",")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rules {
    pub length: usize,
    pub require: Require,
    pub symbols: String,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            length: DEFAULT_LENGTH,
            require: Require::DEFAULT,
            symbols: DEFAULT_SYMBOLS.to_string(),
        }
    }
}

impl Rules {
    /// One validation for CLI flags and for hand-edited profile files (spec 4.2).
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=MAX_LENGTH).contains(&self.length) {
            return Err(format!("length must be between 1 and {MAX_LENGTH}"));
        }
        let total = self.require.total();
        if total == 0 {
            return Err("require must ask for at least one character".into());
        }
        if total > self.length {
            return Err("require total exceeds length".into());
        }
        validate_symbols(&self.symbols)
    }
}

fn validate_symbols(s: &str) -> Result<(), String> {
    let n = s.chars().count();
    if n == 0 || n > MAX_SYMBOLS {
        return Err(format!("symbols must have 1 to {MAX_SYMBOLS} characters"));
    }
    if !s.bytes().all(|b| (0x21..=0x7E).contains(&b) && b != b'|') {
        return Err("symbols must be ASCII 0x21-0x7E without '|'".into());
    }
    let mut seen = [false; 128];
    for b in s.bytes() {
        if std::mem::replace(&mut seen[b as usize], true) {
            return Err("symbols must not repeat a character".into());
        }
    }
    Ok(())
}

pub fn validate_counter(counter: u32) -> Result<(), String> {
    if counter == 0 {
        return Err("counter must be at least 1".into());
    }
    Ok(())
}

fn default_symbols() -> String {
    DEFAULT_SYMBOLS.to_string()
}

/// One saved entry. Identified by the normalized (site, account) pair.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub site: String,
    pub account: String,
    pub length: usize,
    #[serde(default)]
    pub require: Require,
    #[serde(default = "default_symbols")]
    pub symbols: String,
    pub counter: u32,
}

impl Profile {
    pub fn rules(&self) -> Rules {
        Rules { length: self.length, require: self.require, symbols: self.symbols.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(length: usize, c: [usize; 4], symbols: &str) -> Rules {
        Rules { length, require: Require::from_counts(c), symbols: symbols.into() }
    }

    #[test]
    fn default_rules_are_valid() {
        assert_eq!(Rules::default().validate(), Ok(()));
    }

    #[test]
    fn length_bounds() {
        assert!(rules(0, [1, 0, 0, 0], DEFAULT_SYMBOLS).validate().is_err());
        assert!(rules(1, [1, 0, 0, 0], DEFAULT_SYMBOLS).validate().is_ok());
        assert!(rules(128, [1, 0, 0, 0], DEFAULT_SYMBOLS).validate().is_ok());
        assert!(rules(129, [1, 0, 0, 0], DEFAULT_SYMBOLS).validate().is_err());
    }

    #[test]
    fn require_bounds() {
        assert!(rules(10, [0, 0, 0, 0], DEFAULT_SYMBOLS).validate().is_err());
        assert!(rules(4, [1, 1, 1, 1], DEFAULT_SYMBOLS).validate().is_ok());
        assert!(rules(3, [1, 1, 1, 1], DEFAULT_SYMBOLS).validate().is_err());
        assert!(rules(5, [usize::MAX, 1, 0, 0], DEFAULT_SYMBOLS).validate().is_err());
    }

    #[test]
    fn symbols_rules() {
        let ok = |s: &str| rules(8, [1, 0, 0, 1], s).validate();
        assert!(ok("!@").is_ok());
        assert!(ok("").is_err());
        assert!(ok("!!").is_err());
        assert!(ok("!|").is_err());
        assert!(ok("! ").is_err());
        assert!(ok("é").is_err());
        let long: String = (0x21u8..0x21 + 33).map(|b| b as char).filter(|c| *c != '|').collect();
        assert!(long.len() > MAX_SYMBOLS);
        assert!(ok(&long).is_err());
    }

    #[test]
    fn counter_minimum() {
        assert!(validate_counter(0).is_err());
        assert!(validate_counter(1).is_ok());
    }

    #[test]
    fn spec_rendering_skips_zero_classes() {
        assert_eq!(Require::from_counts([1, 1, 2, 0]).to_spec(), "l1,u1,d2");
    }
}
