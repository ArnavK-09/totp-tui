use crate::models::AddAccountForm;

pub enum Screen {
    Accounts,
    AddAccount(AddAccountForm),
    ConfirmDelete(usize),
}
