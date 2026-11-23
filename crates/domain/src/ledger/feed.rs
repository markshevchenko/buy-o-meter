use super::Ledger;
use crate::feed::MainScreen;

impl Ledger {
    /// Свежие дни сверху, внутри дня — по порядковому номеру.
    #[must_use]
    pub fn main_screen(&self) -> MainScreen {
        let _ = self;
        todo!()
    }
}
