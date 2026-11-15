pub mod normalize;

use crate::error::AppError;
use crate::external::argon2_wrapper::argon2id;
use crate::external::sha3_wrapper::{sha3_256, shake256, ByteStream};

// Frozen spec v1 (design doc section 3). Changing any of these changes every password.
pub const CONTEXT_TAG: &str = "pwg-v1";
pub const ARGON_M_KIB: u32 = 65536;
pub const ARGON_T: u32 = 3;
pub const ARGON_P: u32 = 4;

/// Builds "pwg-v1|site|account|counter" from already normalized parts.
pub fn context(site: &str, account: &str, counter: u32) -> String {
    format!("{CONTEXT_TAG}|{site}|{account}|{counter}")
}

/// key + normalized site/account + counter -> SHAKE256 byte stream (section 3.2 to 3.4).
pub fn derive_stream(
    key: &[u8],
    site: &str,
    account: &str,
    counter: u32,
) -> Result<ByteStream, AppError> {
    let salt = sha3_256(context(site, account, counter).as_bytes());
    let dk = argon2id(key, &salt, ARGON_M_KIB, ARGON_T, ARGON_P).map_err(AppError::Kdf)?;
    Ok(shake256(&dk[..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_format() {
        assert_eq!(context("github.com", "a@b.c", 2), "pwg-v1|github.com|a@b.c|2");
    }
}
