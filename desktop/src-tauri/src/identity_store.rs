use keyring::Entry;

const SERVICE: &str = "mepassa";
const USERNAME: &str = "identity.keypair";

pub fn load_identity_b64() -> Result<Option<String>, keyring::Error> {
    let entry = Entry::new(SERVICE, USERNAME)?;
    match entry.get_password() {
        Ok(value) => {
            if value.trim().is_empty() {
                Ok(None)
            } else {
                Ok(Some(value))
            }
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err),
    }
}

pub fn save_identity_b64(value: &str) -> Result<(), keyring::Error> {
    let entry = Entry::new(SERVICE, USERNAME)?;
    entry.set_password(value)
}

pub fn delete_identity() -> Result<(), keyring::Error> {
    let entry = Entry::new(SERVICE, USERNAME)?;
    match entry.delete_password() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err),
    }
}
