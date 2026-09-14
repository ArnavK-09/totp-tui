mod models;
mod storage;
mod utils;

use crate::{models::totp_app::TotpApp, storage::AccountStore};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let store = AccountStore::new()?;
    let accounts = store.load_accounts()?;
    let mut totp = TotpApp::new(accounts, store);

    ratatui::run(|terminal| totp.app(terminal))?;

    Ok(())
}
