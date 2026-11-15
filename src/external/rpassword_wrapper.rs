use super::zeroize_wrapper::Zeroizing;

pub type Key = Zeroizing<String>;

/// Prompts on the TTY (not stdout) and reads one line without echo.
pub fn prompt_key(prompt: &str) -> Result<Key, String> {
    rpassword::prompt_password(prompt)
        .map(Zeroizing::new)
        .map_err(|e| e.to_string())
}
