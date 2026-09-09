use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::{Duration, UNIX_EPOCH};
use std::{io::Error, time::SystemTime};

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

pub fn read_action(mode: InputMode) -> Result<AppAction, Error> {
    if !event::poll(Duration::from_millis(250))? {
        return Ok(AppAction::None);
    }
    let event = event::read()?;

    let Event::Key(key) = event else {
        return Ok(AppAction::None);
    };

    if key.kind != KeyEventKind::Press {
        return Ok(AppAction::None);
    }

    match mode {
        InputMode::Browse => Ok(match key.code {
            KeyCode::Char('q') | KeyCode::Esc => AppAction::Quit,
            KeyCode::Char('a') => AppAction::AddAccount,
            KeyCode::Char('d') => AppAction::DeleteAccount,
            KeyCode::Down | KeyCode::Char('j') => AppAction::SelectNext,
            KeyCode::Up | KeyCode::Char('k') => AppAction::SelectPrevious,
            _ => AppAction::None,
        }),

        InputMode::Form => Ok(match key.code {
            KeyCode::Esc => AppAction::Cancel,
            KeyCode::Enter => AppAction::Submit,
            KeyCode::Tab => AppAction::NextField,
            KeyCode::BackTab => AppAction::PreviousField,
            KeyCode::Backspace => AppAction::Backspace,
            KeyCode::Char(character) => AppAction::Input(character),
            _ => AppAction::None,
        }),

        InputMode::ConfirmDelete => Ok(match key.code {
            KeyCode::Enter | KeyCode::Char('y') => AppAction::ConfirmDelete,
            KeyCode::Esc | KeyCode::Char('n') => AppAction::CancelDelete,
            _ => AppAction::None,
        }),
    }
}

pub fn format_totp_code(code: &str) -> String {
    if code.len() == 6 {
        format!("{} {}", &code[..3], &code[3..])
    } else {
        code.to_owned()
    }
}

pub fn seconds_remaining(period: u32) -> u64 {
    let period = period.max(1) as u64;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    period - (now % period)
}
