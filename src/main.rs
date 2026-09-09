use data_encoding::BASE32_NOPAD;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Paragraph, Widget},
};
use totp_rs::Algorithm;
use uuid::Uuid;
use zeroize::Zeroizing;

mod models;
mod storage;
mod utils;

use crate::{
    models::Account,
    utils::{AppAction, InputMode, read_action},
};
use crate::{
    storage::AccountStore,
    utils::{format_totp_code, seconds_remaining},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum AddAccountField {
    Issuer,
    AccountName,
    Secret,
}

struct AddAccountForm {
    issuer: String,
    account_name: String,
    secret: Zeroizing<String>,
    focused_field: AddAccountField,
    error_message: Option<String>,
}

impl Default for AddAccountForm {
    fn default() -> Self {
        Self {
            issuer: String::new(),
            account_name: String::new(),
            secret: Zeroizing::new(String::new()),
            focused_field: AddAccountField::Issuer,
            error_message: None,
        }
    }
}

impl AddAccountForm {
    fn active_text_mut(&mut self) -> &mut String {
        match self.focused_field {
            AddAccountField::AccountName => &mut self.account_name,
            AddAccountField::Secret => &mut self.secret,
            AddAccountField::Issuer => &mut self.issuer,
        }
    }

    fn normalized_secret(&self) -> String {
        self.secret
            .chars()
            .filter(|character| {
                !character.is_ascii_whitespace() && *character != '-' && *character != '='
            })
            .collect::<String>()
            .to_ascii_uppercase()
    }

    fn validate(&self) -> Result<(), String> {
        if self.issuer.trim().is_empty() {
            return Err("Issuer is required.".to_owned());
        }

        if self.account_name.trim().is_empty() {
            return Err("Account name is required.".to_owned());
        }

        if self.secret.trim().is_empty() {
            return Err("Base32 secret is required.".to_owned());
        }
        let secret = self.normalized_secret();
        if BASE32_NOPAD.decode(secret.as_bytes()).is_err() {
            return Err("The secret must be valid Base32".to_owned());
        }
        Ok(())
    }

    fn push_char(&mut self, character: char) {
        self.active_text_mut().push(character);
        self.error_message = None;
    }

    fn backspace(&mut self) {
        self.active_text_mut().pop();
        self.error_message = None;
    }

    fn focus_next(&mut self) {
        self.focused_field = match self.focused_field {
            AddAccountField::Issuer => AddAccountField::AccountName,
            AddAccountField::AccountName => AddAccountField::Secret,
            AddAccountField::Secret => AddAccountField::Issuer,
        };
    }

    fn focus_previous(&mut self) {
        self.focused_field = match self.focused_field {
            AddAccountField::Issuer => AddAccountField::Secret,
            AddAccountField::AccountName => AddAccountField::Issuer,
            AddAccountField::Secret => AddAccountField::AccountName,
        };
    }
}

enum Screen {
    Accounts,
    AddAccount(AddAccountForm),
    ConfirmDelete(usize),
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let store = AccountStore::new()?;
    let accounts = store.load_accounts()?;
    let mut totp = TotpApp::new(accounts, store);

    ratatui::run(|terminal| totp.app(terminal))?;

    Ok(())
}

struct TotpApp {
    store: AccountStore,
    layout: LayoutManager,
    accounts: Vec<Account>,
    selected_account: usize,
    screen: Screen,
    status_message: Option<String>,
    current_code: Option<String>,
    code_error: Option<String>,
    last_code_step: Option<u64>,
}

impl TotpApp {
    fn new(accounts: Vec<Account>, store: AccountStore) -> Self {
        Self {
            store,
            layout: LayoutManager,
            accounts,
            selected_account: 0,
            screen: Screen::Accounts,
            status_message: None,
            current_code: None,
            code_error: None,
            last_code_step: None,
        }
    }

    fn app(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        loop {
            self.refresh_code_if_needed();
            terminal.draw(|frame| self.render(frame))?;

            let input_mode = match &self.screen {
                Screen::Accounts => InputMode::Browse,
                Screen::AddAccount(_) => InputMode::Form,
                Screen::ConfirmDelete(_) => InputMode::ConfirmDelete,
            };

            match read_action(input_mode)? {
                AppAction::SelectPrevious => self.select_previous(),
                AppAction::SelectNext => self.select_next(),

                AppAction::AddAccount => {
                    self.screen = Screen::AddAccount(AddAccountForm::default());
                }

                AppAction::Input(character) => {
                    if let Screen::AddAccount(form) = &mut self.screen {
                        form.push_char(character);
                    }
                }

                AppAction::Backspace => {
                    if let Screen::AddAccount(form) = &mut self.screen {
                        form.backspace();
                    }
                }

                AppAction::NextField => {
                    if let Screen::AddAccount(form) = &mut self.screen {
                        form.focus_next();
                    }
                }

                AppAction::PreviousField => {
                    if let Screen::AddAccount(form) = &mut self.screen {
                        form.focus_previous();
                    }
                }

                AppAction::Cancel => {
                    self.screen = Screen::Accounts;
                }

                AppAction::Submit => {
                    self.submit_add_account();
                }
                AppAction::DeleteAccount => {
                    if !self.accounts.is_empty() {
                        self.screen = Screen::ConfirmDelete(self.selected_account);
                    }
                }
                AppAction::ConfirmDelete => {
                    self.confirm_delete();
                }
                AppAction::CancelDelete => {
                    self.screen = Screen::Accounts;
                }
                AppAction::Quit => break Ok(()),

                AppAction::None => {}
            }
        }
    }

    fn select_previous(&mut self) {
        if self.accounts.is_empty() {
            return;
        }

        if self.selected_account == 0 {
            self.selected_account = self.accounts.len() - 1;
        } else {
            self.selected_account -= 1;
        }
        self.last_code_step = None;
    }

    fn select_next(&mut self) {
        if self.accounts.is_empty() {
            return;
        }
        self.selected_account = (self.selected_account + 1) % self.accounts.len();
        self.last_code_step = None;
    }

    fn render_form_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        focused: bool,
    ) {
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::bordered().title(label).border_style(border_style);
        let inner = block.inner(area);

        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(value), inner);
    }

    fn render_accounts(&self, frame: &mut Frame) {
        let [content_area, help_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

        let selected_account = self.accounts.get(self.selected_account);
        let (left, right) = self.layout.get_base_layout(content_area);
        let help = Paragraph::new("↑↓ / jk Navigate   a Add   d Delete   q Quit")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));

        frame.render_widget(help, help_area);

        if let Some(account) = selected_account {
            let details = Block::bordered().title(account.issuer.as_str());
            let details_inner = details.inner(right);
            frame.render_widget(details, right);

            let [header_area, code_area, footer_area] = Layout::vertical([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Length(2),
            ])
            .areas(details_inner);

            let header = Paragraph::new(account.account_name.as_str())
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(header, header_area);

            let code_display = match (&self.current_code, &self.code_error) {
                (Some(code), _) => format_totp_code(code),
                (None, Some(error)) => error.clone(),
                (None, None) => "Loading...".to_owned(),
            };

            let code_widget = Paragraph::new(code_display)
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                );
            frame.render_widget(code_widget, code_area);

            let remaining = seconds_remaining(account.period);

            let footer_style = if remaining <= 5 {
                Style::default().fg(Color::LightRed)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let footer_text = match &self.status_message {
                Some(message) => format!("Expires in {:02}s  |  {message}", remaining),
                None => format!("Expires in {:02}s", remaining),
            };

            let footer = Paragraph::new(footer_text)
                .alignment(Alignment::Center)
                .style(footer_style);

            frame.render_widget(footer, footer_area);
        }

        let accounts_block = Block::bordered().title("Accounts");
        let sidebar_inner = accounts_block.inner(left);

        frame.render_widget(accounts_block, left);

        if self.accounts.is_empty() {
            let msg = Paragraph::new("No accounts\n\na  Add account")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));
            let [_, message_area, _] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(3),
                Constraint::Fill(1),
            ])
            .areas(sidebar_inner);

            frame.render_widget(msg, message_area);

            let empty_block = Block::bordered().title("Welcome");
            let empty_inner = empty_block.inner(right);

            frame.render_widget(empty_block, right);

            let [_, message_area, _] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(3),
                Constraint::Fill(1),
            ])
            .areas(empty_inner);

            let message = Paragraph::new("No TOTP accounts yet\n\nAdd an account to begin")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));

            frame.render_widget(message, message_area);
        } else {
            let row_height = 2u16;

            // Number of complete two-line account rows that fit.
            let visible_rows = usize::from(sidebar_inner.height / row_height);

            if visible_rows > 0 {
                // Keep the selected account visible near the bottom while navigating down.
                let max_start = self.accounts.len().saturating_sub(visible_rows);

                let start = self
                    .selected_account
                    .saturating_sub(visible_rows.saturating_sub(1))
                    .min(max_start);

                let visible_count = (self.accounts.len() - start).min(visible_rows);

                let rows = Layout::vertical(vec![Constraint::Length(row_height); visible_count])
                    .split(sidebar_inner);

                for ((index, account), row) in self
                    .accounts
                    .iter()
                    .enumerate()
                    .skip(start)
                    .take(visible_count)
                    .zip(rows.iter())
                {
                    frame.render_widget(
                        AccountListItem {
                            issuer: account.issuer.clone(),
                            account_name: account.account_name.clone(),
                            selected: index == self.selected_account,
                        },
                        *row,
                    );
                }
            }
        }
    }
    fn render_add_account(&self, frame: &mut Frame, form: &AddAccountForm) {
        let block = Block::bordered().title("Add TOTP Account");
        let inner = block.inner(frame.area());

        frame.render_widget(block, frame.area());

        let [
            issuer_area,
            account_name_area,
            secret_area,
            _,
            error_area,
            help_area,
        ] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);

        let masked_secret = "*".repeat(form.secret.len());

        self.render_form_field(
            frame,
            issuer_area,
            "Issuer",
            &form.issuer,
            form.focused_field == AddAccountField::Issuer,
        );

        self.render_form_field(
            frame,
            account_name_area,
            "Account Name",
            &form.account_name,
            form.focused_field == AddAccountField::AccountName,
        );

        self.render_form_field(
            frame,
            secret_area,
            "Base32 Secret",
            &masked_secret,
            form.focused_field == AddAccountField::Secret,
        );

        let help = Paragraph::new("Tab next field   Enter save   Esc cancel")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));

        frame.render_widget(help, help_area);

        if let Some(err) = &form.error_message {
            let error = Paragraph::new(err.as_str())
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::LightRed));
            frame.render_widget(error, error_area);
        }
    }

    fn render_confirm_delete(&self, frame: &mut Frame, id: usize) {
        let block = Block::bordered().title("Confirm Delete?");
        let inner = block.inner(frame.area());

        frame.render_widget(block, frame.area());

        let message = match self.accounts.get(id) {
            Some(account) => format!(
                "Delete {} / {}?\n\nEnter or y: delete\nEsc or n: cancel",
                account.issuer, account.account_name,
            ),
            None => "Account no longer exists.\n\nPress Esc to cancel.".to_owned(),
        };

        let paragraph = Paragraph::new(message)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(paragraph, inner);
    }

    fn render(&self, frame: &mut Frame) {
        match &self.screen {
            Screen::Accounts => self.render_accounts(frame),
            Screen::AddAccount(form) => self.render_add_account(frame, form),
            Screen::ConfirmDelete(i) => self.render_confirm_delete(frame, *i),
        }
    }

    fn selected_account(&self) -> Option<&Account> {
        self.accounts.get(self.selected_account)
    }

    fn refresh_code_if_needed(&mut self) {
        if !matches!(&self.screen, Screen::Accounts) {
            return;
        }

        let Some(period) = self.selected_account().map(|account| account.period) else {
            self.current_code = None;
            self.code_error = None;
            self.last_code_step = None;
            return;
        };

        let period = period.max(1) as u64;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let current_step = now / period;

        if self.last_code_step == Some(current_step) {
            return;
        }

        self.refresh_selected_code();
        self.last_code_step = Some(current_step);
    }

    fn refresh_selected_code(&mut self) {
        match self.generate_selected_code() {
            Ok(code) => {
                self.current_code = Some(code);
                self.code_error = None;
            }
            Err(_) => {
                self.current_code = None;
                self.code_error = Some("Unable to read the account secret.".to_owned());
            }
        }
    }

    fn confirm_delete(&mut self) {
        let i = match &self.screen {
            Screen::ConfirmDelete(index) => *index,
            _ => return,
        };

        let Some(account_id) = self.accounts.get(i).map(|a| a.id.clone()) else {
            self.status_message = Some("Account no longer exists".to_owned());
            self.screen = Screen::Accounts;
            return;
        };
        match self.store.delete_account(&self.accounts, &account_id) {
            Ok(remaining_accounts) => {
                self.accounts = remaining_accounts;

                if self.selected_account >= self.accounts.len() {
                    self.selected_account = self.accounts.len().saturating_sub(1);
                }

                self.current_code = None;
                self.code_error = None;
                self.last_code_step = None;
                self.screen = Screen::Accounts;
            }
            Err(e) => {
                self.status_message = Some(format!("Delete failed: {e}"));
                self.screen = Screen::Accounts;
            }
        };
    }

    fn submit_add_account(&mut self) {
        let Screen::AddAccount(mut form) = std::mem::replace(&mut self.screen, Screen::Accounts)
        else {
            return;
        };

        if let Err(message) = form.validate() {
            form.error_message = Some(message);
            self.screen = Screen::AddAccount(form);
            return;
        }

        let secret = form.normalized_secret();

        let account = Account {
            id: Uuid::new_v4().to_string(),
            issuer: form.issuer.trim().to_owned(),
            account_name: form.account_name.trim().to_owned(),
            period: 30,
            digits: 6,
        };

        match self
            .store
            .save_new_account(&self.accounts, &account, &secret)
        {
            Ok(()) => {
                self.accounts.push(account);
                self.selected_account = self.accounts.len() - 1;
                self.status_message = Some("Account added.".to_owned());
                self.screen = Screen::Accounts;
            }
            Err(error) => {
                form.error_message = Some(format!("Save failed: {error}"));
                self.screen = Screen::AddAccount(form);
            }
        }
    }

    fn generate_selected_code(&self) -> color_eyre::Result<String> {
        let acc = self
            .selected_account()
            .ok_or_else(|| color_eyre::eyre::eyre!("No account selected"))?;
        let secret = self.store.load_secret(&acc.id)?;
        let secret_bytes = BASE32_NOPAD.decode(secret.as_bytes())?;
        let totp = totp_rs::Builder::new()
            .with_algorithm(Algorithm::SHA1)
            .with_digits(acc.digits as u8)
            .with_skew(1)
            .with_step_duration(acc.period as u64)
            .with_secret(secret_bytes)
            .build()?;

        Ok(totp.generate_current().to_string())
    }
}

struct LayoutManager;

impl LayoutManager {
    fn get_base_layout(&self, area: Rect) -> (Rect, Rect) {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(20), Constraint::Fill(1)]).areas(area);

        (left, right)
    }
}

pub struct AccountListItem {
    issuer: String,
    account_name: String,
    selected: bool,
}

impl Widget for AccountListItem {
    fn render(self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let selected_style = Style::default().fg(Color::Black).bg(Color::Cyan);

        let issuer_style = if self.selected {
            selected_style
        } else {
            Style::default().fg(Color::White)
        };

        let account_style = if self.selected {
            selected_style
        } else {
            Style::default().fg(Color::DarkGray)
        };

        if self.selected {
            buf.set_style(area, selected_style);
        }

        let marker = if self.selected { "›" } else { " " };

        buf.set_string(area.left(), area.top(), marker, issuer_style);

        buf.set_string(area.left() + 2, area.top(), &self.issuer, issuer_style);

        buf.set_string(
            area.left() + 2,
            area.top() + 1,
            &self.account_name,
            account_style,
        );
    }
}
