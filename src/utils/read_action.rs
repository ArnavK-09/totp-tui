use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io::Error;
use std::time::Duration;

use crate::utils::{AppAction, InputMode};

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
