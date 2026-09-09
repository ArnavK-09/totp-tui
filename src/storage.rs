use directories::ProjectDirs;
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
};
use tempfile::NamedTempFile;
use zeroize::Zeroizing;

use crate::models::Account;

pub struct AccountStore {
    metadata_path: PathBuf,
    keyring_service: String,
}

const KEYRING_SERVICE: &str = "com.ArnavK-09.totp-tui";

impl AccountStore {
    pub fn new() -> io::Result<Self> {
        let dirs = ProjectDirs::from("com", "ArnavK-09", "totp-tui")
            .ok_or_else(|| io::Error::other("Could not determine config directory"))?;
        let config_dir = dirs.config_dir();

        fs::create_dir_all(config_dir)?;

        Ok(Self {
            metadata_path: config_dir.join("accounts.json"),
            keyring_service: KEYRING_SERVICE.to_owned(),
        })
    }

    pub fn save_secret(&self, account_id: &str, secret: &str) -> color_eyre::Result<()> {
        let entry = keyring::Entry::new(&self.keyring_service, account_id)?;
        entry.set_password(secret)?;
        Ok(())
    }
    pub fn delete_secret(&self, account_id: &str) -> color_eyre::Result<()> {
        let entry = keyring::Entry::new(&self.keyring_service, account_id)?;
        entry.delete_credential()?;
        Ok(())
    }

    pub fn load_secret(&self, account_id: &str) -> color_eyre::Result<String> {
        let a = keyring::Entry::new(&self.keyring_service, account_id)?;
        Ok(a.get_password()?)
    }

    pub fn save_new_account(
        &self,
        existing_accounts: &[Account],
        account: &Account,
        secret: &str,
    ) -> color_eyre::Result<()> {
        self.save_secret(&account.id, secret)
            .map_err(|error| color_eyre::eyre::eyre!("keyring write failed: {error}"))?;

        if let Err(error) = self.load_secret(&account.id) {
            let _ = self.delete_secret(&account.id);

            return Err(color_eyre::eyre::eyre!("keyring read-back failed: {error}"));
        }

        let mut next_accounts = existing_accounts.to_vec();
        next_accounts.push(account.clone());

        if let Err(error) = self.save_accounts(&next_accounts) {
            // Avoid leaving an orphaned secret if metadata saving fails.
            let _ = self.delete_secret(&account.id);

            return Err(color_eyre::eyre::eyre!("metadata write failed: {error}"));
        }

        Ok(())
    }

    pub fn save_accounts(&self, accounts: &[Account]) -> io::Result<()> {
        let contents = serde_json::to_vec_pretty(accounts).map_err(io::Error::other)?;

        let directory = self
            .metadata_path
            .parent()
            .ok_or_else(|| io::Error::other("Metadata path has no parent directory"))?;

        let mut temporary_file = NamedTempFile::new_in(directory)?;

        temporary_file.write_all(&contents)?;
        temporary_file.as_file().sync_all()?;

        temporary_file
            .persist(&self.metadata_path)
            .map_err(|error| error.error)?;

        Ok(())
    }

    pub fn delete_account(
        &self,
        existing_accounts: &[Account],
        account_id: &str,
    ) -> color_eyre::Result<Vec<Account>> {
        let account_exists = existing_accounts
            .iter()
            .any(|account| account.id == account_id);

        if !account_exists {
            return Err(color_eyre::eyre::eyre!("Account metadata was not found"));
        }

        let secret = Zeroizing::new(self.load_secret(account_id)?);

        self.delete_secret(account_id)?;

        let remaining_accounts: Vec<Account> = existing_accounts
            .iter()
            .filter(|account| account.id != account_id)
            .cloned()
            .collect();

        if let Err(metadata_error) = self.save_accounts(&remaining_accounts) {
            if let Err(restore_error) = self.save_secret(account_id, secret.as_str()) {
                return Err(color_eyre::eyre::eyre!(
                    "Metadata save failed and secret restoration failed: {restore_error}"
                ));
            }

            return Err(color_eyre::eyre::eyre!(
                "Metadata save failed; account was not deleted: {metadata_error}"
            ));
        }

        Ok(remaining_accounts)
    }

    pub fn load_accounts(&self) -> io::Result<Vec<Account>> {
        if !self.metadata_path.exists() {
            return Ok(Vec::new());
        };
        let content = fs::read_to_string(&self.metadata_path)?;
        serde_json::from_str(&content).map_err(io::Error::other)
    }
}
