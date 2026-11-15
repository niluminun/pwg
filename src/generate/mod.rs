use crate::external::sha3_wrapper::ByteStream;
use crate::profile::rules::Rules;

// Frozen alphabets (spec 3.6). Character order inside each string is part of the spec.
const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGIT: &str = "0123456789";

/// Maps one 16-bit draw to 0..n, or None when it falls in the biased tail.
pub fn below_from_u16(v: u16, n: usize) -> Option<usize> {
    let limit = 65536 - (65536 % n);
    let v = v as usize;
    (v < limit).then_some(v % n)
}

/// Uniform value in 0..n. Always reads 2 bytes per attempt, even when n == 1 (frozen spec 3.5).
pub fn below(stream: &mut ByteStream, n: usize) -> usize {
    loop {
        let mut b = [0u8; 2];
        stream.read(&mut b);
        if let Some(x) = below_from_u16(u16::from_be_bytes(b), n) {
            return x;
        }
    }
}

struct Class {
    alphabet: Vec<u8>,
    min: usize,
}

/// Classes in fixed order lower, upper, digit, symbol; only those with min > 0.
fn used_classes(rules: &Rules) -> Vec<Class> {
    let [l, u, d, s] = rules.require.counts();
    [
        (LOWER.as_bytes().to_vec(), l),
        (UPPER.as_bytes().to_vec(), u),
        (DIGIT.as_bytes().to_vec(), d),
        (rules.symbols.as_bytes().to_vec(), s),
    ]
    .into_iter()
    .filter(|(_, min)| *min > 0)
    .map(|(alphabet, min)| Class { alphabet, min })
    .collect()
}

/// Builds the password from the stream. Draw order is part of the frozen spec (3.6).
pub fn generate(stream: &mut ByteStream, rules: &Rules) -> String {
    let classes = used_classes(rules);
    let union: Vec<u8> = classes.iter().flat_map(|c| c.alphabet.iter().copied()).collect();
    let mut out: Vec<u8> = Vec::with_capacity(rules.length);

    // 1. Required characters, class by class.
    for c in &classes {
        for _ in 0..c.min {
            out.push(c.alphabet[below(stream, c.alphabet.len())]);
        }
    }
    // 2. Remaining slots from the union alphabet.
    while out.len() < rules.length {
        out.push(union[below(stream, union.len())]);
    }
    // 3. Fisher-Yates shuffle, from the last index down to 1.
    for i in (1..out.len()).rev() {
        let j = below(stream, i + 1);
        out.swap(i, j);
    }
    out.into_iter().map(char::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::sha3_wrapper::shake256;
    use crate::profile::rules::Require;

    #[test]
    fn below_rejects_biased_tail() {
        // n = 10: limit = 65536 - 6 = 65530.
        assert_eq!(below_from_u16(65529, 10), Some(9));
        assert_eq!(below_from_u16(65530, 10), None);
        assert_eq!(below_from_u16(65535, 10), None);
        assert_eq!(below_from_u16(0, 10), Some(0));
    }

    #[test]
    fn below_power_of_two_never_rejects() {
        assert_eq!(below_from_u16(65535, 64), Some(63));
        assert_eq!(below_from_u16(65535, 1), Some(0));
    }

    #[test]
    fn generate_meets_rules() {
        let rules = Rules {
            length: 24,
            require: Require::from_counts([2, 3, 4, 5]),
            symbols: "-_".into(),
        };
        let pw = generate(&mut shake256(b"seed"), &rules);
        assert_eq!(pw.len(), 24);
        assert!(pw.bytes().filter(u8::is_ascii_lowercase).count() >= 2);
        assert!(pw.bytes().filter(u8::is_ascii_uppercase).count() >= 3);
        assert!(pw.bytes().filter(u8::is_ascii_digit).count() >= 4);
        assert!(pw.bytes().filter(|b| *b == b'-' || *b == b'_').count() >= 5);
    }

    #[test]
    fn unused_class_never_appears() {
        let rules = Rules {
            length: 128,
            require: Require::from_counts([1, 1, 1, 0]),
            symbols: "!@".into(),
        };
        let pw = generate(&mut shake256(b"seed"), &rules);
        assert!(pw.bytes().all(|b| b.is_ascii_alphanumeric()));
    }
}
