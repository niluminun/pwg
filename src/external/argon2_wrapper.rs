use argon2::{Algorithm, Argon2, Params, Version};

use super::zeroize_wrapper::Zeroizing;

/// Argon2id v0x13 with 32-byte output, no secret and no associated data.
pub fn argon2id(
    key: &[u8],
    salt: &[u8],
    m_kib: u32,
    t: u32,
    p: u32,
) -> Result<Zeroizing<[u8; 32]>, String> {
    let params = Params::new(m_kib, t, p, Some(32)).map_err(|e| e.to_string())?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(key, salt, &mut *out)
        .map_err(|e| e.to_string())?;
    Ok(out)
}
