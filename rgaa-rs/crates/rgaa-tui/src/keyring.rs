const SERVICE: &str = "rgaa";

const DEFAULT_BASE_URL: &str = "https://api.hcompany.ai/v1/chat/completions";

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
    let existing_url = get_base_url_from_fallback()
        .or_else(|| os_keyring_get_base_url().ok())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    fallback_store(key, &existing_url)
}

pub fn get_api_key() -> Result<Option<String>, KeyringError> {
    if let Ok(key) = os_keyring_get_api_key() {
        if !key.is_empty() {
            return Ok(Some(key));
        }
    }
    fallback_get_api_key()
}

pub fn get_base_url() -> Option<String> {
    os_keyring_get_base_url()
        .ok()
        .or_else(get_base_url_from_fallback)
}

pub fn store_base_url(url: &str) -> Result<(), KeyringError> {
    let entry = keyring::Entry::new(SERVICE, "holo3_base_url")
        .map_err(|e| KeyringError::Keyring(e.to_string()))?;
    if entry.set_password(url).is_ok() {
        return Ok(());
    }
    let existing_key = get_api_key_from_fallback()
        .or_else(|| os_keyring_get_api_key().ok())
        .unwrap_or_default();
    fallback_store_api_key_and_url(&existing_key, url)
}

fn os_keyring_get_api_key() -> Result<String, KeyringError> {
    let entry = keyring::Entry::new(SERVICE, "holo3_api_key")
        .map_err(|e| KeyringError::Keyring(e.to_string()))?;
    entry
        .get_password()
        .map_err(|e| KeyringError::Keyring(e.to_string()))
}

fn os_keyring_get_base_url() -> Result<String, KeyringError> {
    let entry = keyring::Entry::new(SERVICE, "holo3_base_url")
        .map_err(|e| KeyringError::Keyring(e.to_string()))?;
    entry
        .get_password()
        .map_err(|e| KeyringError::Keyring(e.to_string()))
}

fn fallback_store_api_key_and_url(key: &str, base_url: &str) -> Result<(), KeyringError> {
    let home = dirs::home_dir().ok_or_else(|| {
        KeyringError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "no home dir"))
    })?;
    let env_path = home.join(".rgaa").join("env");
    std::fs::create_dir_all(env_path.parent().unwrap())?;
    let content = format!("HOLO3_API_KEY={}\nHOLO3_BASE_URL={}\n", key, base_url);
    std::fs::write(&env_path, content)?;
    eprintln!(
        "WARNING: OS keyring unavailable. Config stored in plain text at ~/.rgaa/env"
    );
    Ok(())
}

fn fallback_store(key: &str, base_url: &str) -> Result<(), KeyringError> {
    fallback_store_api_key_and_url(key, base_url)
}

fn fallback_get_api_key() -> Result<Option<String>, KeyringError> {
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

fn get_base_url_from_fallback() -> Option<String> {
    let home = dirs::home_dir()?;
    let env_path = home.join(".rgaa").join("env");
    let content = std::fs::read_to_string(&env_path).ok()?;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("HOLO3_BASE_URL=") {
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

fn get_api_key_from_fallback() -> Option<String> {
    let home = dirs::home_dir()?;
    let env_path = home.join(".rgaa").join("env");
    let content = std::fs::read_to_string(&env_path).ok()?;
    for line in content.lines() {
        if let Some(val) = line.strip_prefix("HOLO3_API_KEY=") {
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}
