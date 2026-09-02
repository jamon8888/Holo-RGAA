const SERVICE: &str = "rgaa";

#[derive(Debug, thiserror::Error)]
pub enum KeyringError {
    #[error("keyring error: {0}")]
    Keyring(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn store_api_key(key: &str) -> Result<(), KeyringError> {
    let entry = keyring::Entry::new(SERVICE, "holo3_api_key")
        .map_err(|e| KeyringError::Keyring(e.to_string()))?;
    if entry.set_password(key).is_ok() {
        return Ok(());
    }
    fallback_store(key)
}

pub fn get_api_key() -> Result<Option<String>, KeyringError> {
    let entry = keyring::Entry::new(SERVICE, "holo3_api_key")
        .map_err(|e| KeyringError::Keyring(e.to_string()))?;
    match entry.get_password() {
        Ok(pw) if !pw.is_empty() => return Ok(Some(pw)),
        _ => {}
    }
    fallback_get()
}

fn fallback_store(key: &str) -> Result<(), KeyringError> {
    let home = dirs::home_dir().ok_or_else(|| {
        KeyringError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "no home dir"))
    })?;
    let env_path = home.join(".rgaa").join("env");
    std::fs::create_dir_all(env_path.parent().unwrap())?;
    let content = format!(
        "HOLO3_API_KEY={}\nHOLO3_BASE_URL=https://api.holo3.ai/v1\n",
        key
    );
    std::fs::write(&env_path, content)?;
    eprintln!(
        "WARNING: OS keyring unavailable. API key stored in plain text at ~/.rgaa/env"
    );
    Ok(())
}

fn fallback_get() -> Result<Option<String>, KeyringError> {
    if let Some(home) = dirs::home_dir() {
        let env_path = home.join(".rgaa").join("env");
        if env_path.exists() {
            let content = std::fs::read_to_string(&env_path)?;
            for line in content.lines() {
                if let Some(val) = line.strip_prefix("HOLO3_API_KEY=") {
                    if !val.is_empty() {
                        return Ok(Some(val.to_string()));
                    }
                }
            }
        }
    }
    Ok(None)
}
