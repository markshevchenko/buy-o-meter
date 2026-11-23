use super::Ledger;
use crate::feed::MainScreen;

impl Ledger {
    /// Свежие дни сверху, внутри дня — по порядковому номеру.
    #[must_use]
    pub fn main_screen(&self) -> MainScreen {
        MainScreen::new(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::super::Ledger;

    #[test]
    fn empty_ledger_has_empty_main_screen() {
        let ledger = Ledger::new();
        let screen = ledger.main_screen();
        assert!(screen.is_empty());
        assert!(screen.days().is_empty());
    }
}
