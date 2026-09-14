use data_encoding::BASE32_NOPAD;
use zeroize::Zeroizing;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AddAccountField {
    Issuer,
    AccountName,
    Secret,
}

pub struct AddAccountForm {
    pub issuer: String,
    pub account_name: String,
    pub secret: Zeroizing<String>,
    pub focused_field: AddAccountField,
    pub error_message: Option<String>,
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
    pub fn active_text_mut(&mut self) -> &mut String {
        match self.focused_field {
            AddAccountField::AccountName => &mut self.account_name,
            AddAccountField::Secret => &mut self.secret,
            AddAccountField::Issuer => &mut self.issuer,
        }
    }

    pub fn normalized_secret(&self) -> String {
        self.secret
            .chars()
            .filter(|character| {
                !character.is_ascii_whitespace() && *character != '-' && *character != '='
            })
            .collect::<String>()
            .to_ascii_uppercase()
    }

    pub fn validate(&self) -> Result<(), String> {
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

    pub fn push_char(&mut self, character: char) {
        self.active_text_mut().push(character);
        self.error_message = None;
    }

    pub fn backspace(&mut self) {
        self.active_text_mut().pop();
        self.error_message = None;
    }

    pub fn focus_next(&mut self) {
        self.focused_field = match self.focused_field {
            AddAccountField::Issuer => AddAccountField::AccountName,
            AddAccountField::AccountName => AddAccountField::Secret,
            AddAccountField::Secret => AddAccountField::Issuer,
        };
    }

    pub fn focus_previous(&mut self) {
        self.focused_field = match self.focused_field {
            AddAccountField::Issuer => AddAccountField::Secret,
            AddAccountField::AccountName => AddAccountField::Issuer,
            AddAccountField::Secret => AddAccountField::AccountName,
        };
    }
}
