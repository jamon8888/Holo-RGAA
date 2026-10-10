const SERVICE: &str = "rgaa";

const API_KEY_ITEM: &str = "myia_api_key";
const BASE_URL_ITEM: &str = "myia_base_url";
const DEFAULT_BASE_URL: &str = "https://api.medium.text-generation-webui.myia.io/v1";

#[derive(Debug, thiserror::Error)]
pub enum KeyringError {
    #[error("keyring error: {0}")]
    Keyring(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn store_api_key(key: &str) -> Result<(), KeyringError> {
    let entry = keyring::Entry::new(SERVICE, API_KEY_ITEM)
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

/// Reads a stored Holo3 key without treating it as a MyIA credential.
pub fn get_legacy_holo3_api_key() -> Result<Option<String>, KeyringError> {
    if let Ok(key) = os_keyring_get_legacy_holo3_api_key() {
        if !key.is_empty() {
            return Ok(Some(key));
        }
    }
    fallback_get_legacy_holo3_api_key()
}

pub fn get_base_url() -> Option<String> {
    os_keyring_get_base_url()
        .ok()
        .or_else(get_base_url_from_fallback)
}

pub fn store_base_url(url: &str) -> Result<(), KeyringError> {
    let entry = keyring::Entry::new(SERVICE, BASE_URL_ITEM)
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
    let entry = keyring::Entry::new(SERVICE, API_KEY_ITEM)
        .map_err(|e| KeyringError::Keyring(e.to_string()))?;
    entry
        .get_password()
        .map_err(|e| KeyringError::Keyring(e.to_string()))
}

fn os_keyring_get_legacy_holo3_api_key() -> Result<String, KeyringError> {
    let entry = keyring::Entry::new(SERVICE, "holo3_api_key")
        .map_err(|e| KeyringError::Keyring(e.to_string()))?;
    entry
        .get_password()
        .map_err(|e| KeyringError::Keyring(e.to_string()))
}

fn os_keyring_get_base_url() -> Result<String, KeyringError> {
    let entry = keyring::Entry::new(SERVICE, BASE_URL_ITEM)
        .map_err(|e| KeyringError::Keyring(e.to_string()))?;
    entry
        .get_password()
        .map_err(|e| KeyringError::Keyring(e.to_string()))
}

fn fallback_store_api_key_and_url(key: &str, base_url: &str) -> Result<(), KeyringError> {
    let home = dirs::home_dir().ok_or_else(|| {
        KeyringError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no home dir",
        ))
    })?;
    let env_path = home.join(".rgaa").join("env");
    std::fs::create_dir_all(env_path.parent().unwrap())?;
    let existing = match std::fs::read_to_string(&env_path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    let mut content = existing
        .lines()
        .filter(|line| !line.starts_with("MYIA_API_KEY=") && !line.starts_with("MYIA_BASE_URL="))
        .collect::<Vec<_>>()
        .join("\n");
    if !content.is_empty() {
        content.push('\n');
    }
    content.push_str(&format!("MYIA_API_KEY={key}\nMYIA_BASE_URL={base_url}\n"));
    std::fs::write(&env_path, content)?;
    eprintln!("WARNING: OS keyring unavailable. Config stored in plain text at ~/.rgaa/env");
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
                if let Some(val) = line.strip_prefix("MYIA_API_KEY=") {
                    if !val.is_empty() {
                        return Ok(Some(val.to_string()));
                    }
                }
            }
        }
    }
    Ok(None)
}

fn fallback_get_legacy_holo3_api_key() -> Result<Option<String>, KeyringError> {
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
        if let Some(val) = line.strip_prefix("MYIA_BASE_URL=") {
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
        if let Some(val) = line.strip_prefix("MYIA_API_KEY=") {
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}
