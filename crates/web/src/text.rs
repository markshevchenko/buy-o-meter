use std::time::{SystemTime, UNIX_EPOCH};

use domain::{DomainError, FeedLine, FeedMoney, OperationKind, RepaymentDirection};

pub(crate) fn kind_name(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::Expense => "Расход",
        OperationKind::Income => "Доход",
        OperationKind::Transfer => "Перевод",
        OperationKind::Lend => "Дать в долг",
        OperationKind::Borrow => "Взять в долг",
        OperationKind::Repayment => "Возврат",
    }
}

pub(crate) fn kind_from_name(name: &str) -> Option<OperationKind> {
    OperationKind::all()
        .iter()
        .copied()
        .find(|kind| kind_name(*kind) == name)
}

pub(crate) fn line_text(line: &FeedLine) -> String {
    match line {
        FeedLine::Expense {
            money,
            category_name,
            ..
        } => format!("Расход · {} · {category_name}", money_text(money)),
        FeedLine::Income {
            money,
            category_name,
            ..
        } => format!("Доход · {} · {category_name}", money_text(money)),
        FeedLine::Transfer { from, to, .. } => transfer_text(from, to),
        FeedLine::Lend {
            money,
            counterparty,
            ..
        } => format!("Дать в долг · {} · {counterparty}", money_text(money)),
        FeedLine::Borrow {
            money,
            counterparty,
            ..
        } => format!("Взять в долг · {} · {counterparty}", money_text(money)),
        FeedLine::Repayment {
            money,
            counterparty,
            direction,
            ..
        } => format!(
            "Возврат · {} · {} · {counterparty}",
            direction_name(*direction),
            money_text(money)
        ),
    }
}

pub(crate) fn format_day(time: SystemTime) -> String {
    let Some(date) = utc_date(time) else {
        return String::new();
    };
    format!(
        "{:02}.{:02}.{}",
        date.day(),
        u8::from(date.month()),
        date.year()
    )
}

pub(crate) fn input_date(time: SystemTime) -> String {
    let Some(date) = utc_date(time) else {
        return String::new();
    };
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

pub(crate) fn today_input() -> String {
    let (year, month, day) = today_parts();
    format!("{year:04}-{month:02}-{day:02}")
}

pub(crate) fn parse_date(text: &str) -> Result<SystemTime, DomainError> {
    let mut parts = text.split('-');
    let year = parse_part(parts.next())?;
    let month = parse_part(parts.next())?;
    let day = parse_part(parts.next())?;
    domain::date(year, month, day)
}

pub(crate) fn error_text(error: &DomainError) -> &'static str {
    match error {
        DomainError::AccountRequired | DomainError::AccountNotFound => "Нужен счёт",
        DomainError::CategoryNotFound => "Нужна категория",
        DomainError::MissingFields(_) => "Заполните счёт, категорию и сумму",
        DomainError::MissingTransferFields(_) => "Заполните счета и суммы перевода",
        DomainError::MissingDebtFields(_) => "Заполните счёт, контрагента и сумму",
        DomainError::MissingRepaymentFields(_) => "Заполните возврат долга",
        DomainError::NoDebt => "Нет такого долга",
        DomainError::RepaymentExceedsDebt { .. } => "Сумма больше остатка долга",
        DomainError::SameAccount => "Выберите разные счета",
        DomainError::InvalidDate => "Некорректная дата",
        DomainError::OperationNotFound => "Операция не найдена",
        DomainError::SubsequentRepaymentsDoNotMatch => "Возвраты не сходятся с долгом",
        _ => "Не удалось сохранить",
    }
}

fn money_text(money: &FeedMoney) -> String {
    format!(
        "{} {} · {}",
        money.amount(),
        money.currency().as_str(),
        money.account_name()
    )
}

fn transfer_text(from: &FeedMoney, to: &FeedMoney) -> String {
    if from.currency() == to.currency() {
        return format!(
            "Перевод · {} {} · {} → {}",
            from.amount(),
            from.currency().as_str(),
            from.account_name(),
            to.account_name()
        );
    }
    format!("Перевод · {} → {}", money_text(from), money_text(to))
}

fn direction_name(direction: RepaymentDirection) -> &'static str {
    match direction {
        RepaymentDirection::Received => "мне вернули",
        RepaymentDirection::Given => "я вернул",
    }
}

fn utc_date(time: SystemTime) -> Option<time::Date> {
    let seconds = system_seconds(time)?;
    time::UtcDateTime::from_unix_timestamp(seconds)
        .ok()
        .map(|moment| moment.date())
}

fn system_seconds(time: SystemTime) -> Option<i64> {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => i64::try_from(duration.as_secs()).ok(),
        Err(error) => {
            let seconds = i64::try_from(error.duration().as_secs()).ok()?;
            Some(-seconds)
        }
    }
}

fn parse_part(part: Option<&str>) -> Result<i32, DomainError> {
    part.unwrap_or("")
        .parse()
        .map_err(|_| DomainError::InvalidDate)
}

fn today_parts() -> (i32, i32, u8) {
    #[cfg(target_arch = "wasm32")]
    {
        wasm_today()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        native_today()
    }
}

#[cfg(target_arch = "wasm32")]
fn wasm_today() -> (i32, i32, u8) {
    let now = js_sys::Date::new_0();
    let year = now.get_full_year() as i32;
    let month = now.get_month() as i32 + 1;
    let day = u8::try_from(now.get_date() as i64).unwrap_or(1);
    (year, month, day)
}

#[cfg(not(target_arch = "wasm32"))]
fn native_today() -> (i32, i32, u8) {
    utc_date(SystemTime::now())
        .map(|date| (date.year(), i32::from(u8::from(date.month())), date.day()))
        .unwrap_or((1970, 1, 1))
}
