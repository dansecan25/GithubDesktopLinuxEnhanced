//! The GitHub token, kept in the system keychain (GNOME Keyring or KWallet through the Secret
//! Service on Linux, Keychain on macOS). It is never written to the settings file.

use keyring::{Entry, Error};

use crate::constants::github::{KEYRING_ACCOUNT, KEYRING_SERVICE};

fn entry() -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).map_err(|error| describe(&error))
}

fn describe(error: &Error) -> String {
    match error {
        Error::NoStorageAccess(_) | Error::PlatformFailure(_) | Error::NoDefaultStore => {
            format!("the system keychain isn't available ({error})")
        }
        _ => error.to_string(),
    }
}

pub fn save_token(token: &str) -> Result<(), String> {
    entry()?.set_password(token).map_err(|error| describe(&error))
}

/// `Ok(None)` when no token is saved.
pub fn load_token() -> Result<Option<String>, String> {
    match entry()?.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(Error::NoEntry) => Ok(None),
        Err(error) => Err(describe(&error)),
    }
}

/// Deleting a token that isn't there counts as success.
pub fn delete_token() -> Result<(), String> {
    match entry()?.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(error) => Err(describe(&error)),
    }
}
