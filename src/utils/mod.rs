pub enum AppAction {
    Quit,
    SelectNext,
    SelectPrevious,
    AddAccount,
    Input(char),
    Backspace,
    NextField,
    PreviousField,
    Submit,
    Cancel,
    DeleteAccount,
    ConfirmDelete,
    CancelDelete,
    None,
}

pub enum InputMode {
    Browse,
    Form,
    ConfirmDelete,
}

pub mod format_totp_code;
pub mod read_action;
pub mod seconds_remaining;

pub use format_totp_code::format_totp_code;
pub use read_action::read_action;
pub use seconds_remaining::seconds_remaining;
