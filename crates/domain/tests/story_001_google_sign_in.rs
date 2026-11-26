mod common;

use common::{create_account, create_top};
use domain::{
    APPDATA_SCOPE, Applicability, AuthError, Category, CategoryId, Ledger, MemoryDrive,
    MemoryIdentity, NewExpense, OperationEntry, OperationForm, OperationKind, Place,
    SIGN_IN_REQUIRED, Session, SessionError, UpdateAccount, date,
};

fn category_fingerprint(
    ledger: &Ledger,
    category: &Category,
) -> (String, Option<String>, Applicability) {
    let parent = category.parent_id().map(|id| {
        ledger
            .category(id)
            .unwrap_or_else(|_| panic!("parent category"))
            .name()
            .to_owned()
    });
    (category.name().to_owned(), parent, category.applicability())
}

fn assert_categories_match_starter(ledger: &Ledger) {
    let mut expected = Ledger::new();
    expected.seed_starter_categories();
    assert_eq!(ledger.categories().len(), expected.categories().len());
    for (actual, baseline) in ledger.categories().iter().zip(expected.categories()) {
        assert_eq!(
            category_fingerprint(ledger, actual),
            category_fingerprint(&expected, baseline)
        );
    }
}

fn require_child_category(ledger: &Ledger, name: &str) -> CategoryId {
    ledger
        .categories()
        .iter()
        .find(|category| category.name() == name && category.parent_id().is_some())
        .map(Category::id)
        .unwrap_or_else(|| panic!("category {name} is missing"))
}

fn operation_comments(ledger: &Ledger) -> Vec<String> {
    ledger
        .operations()
        .iter()
        .filter_map(|entry| {
            if entry.kind() != OperationKind::Expense {
                return None;
            }
            let form = ledger.operation_form(entry.id()).ok()?;
            match form {
                OperationForm::Expense(expense) => Some(expense.comment),
                _ => None,
            }
        })
        .collect()
}

fn sign_in_to_main(
    session: &mut Session<MemoryIdentity, MemoryDrive>,
) -> Result<Place, SessionError> {
    pollster::block_on(session.open());
    pollster::block_on(session.sign_in())
}

// 001 «Первый вход»
#[test]
fn fresh_device_sees_sign_in_without_ledger_even_when_drive_has_data() -> Result<(), SessionError> {
    let drive = MemoryDrive::new();
    let first_identity = MemoryIdentity::new();
    let mut first = Session::new(first_identity, drive.clone());
    sign_in_to_main(&mut first)?;
    pollster::block_on(first.change(|ledger| {
        create_account(ledger, "Наличные", "RUB", 0)?;
        Ok(())
    }))?;

    let second_identity = MemoryIdentity::new();
    let mut second = Session::new(second_identity, drive);
    let place = pollster::block_on(second.open());
    assert_eq!(place, Place::SignIn { message: None });
    assert_eq!(second.place(), Place::SignIn { message: None });
    assert!(second.ledger().is_none());
    Ok(())
}

// 001 «Разрешение на хранение»
#[test]
fn sign_in_requests_only_app_data_scope() -> Result<(), SessionError> {
    let identity = MemoryIdentity::new();
    let mut session = Session::new(identity.clone(), MemoryDrive::new());
    sign_in_to_main(&mut session)?;
    let scopes = session.identity().requested_scopes();
    assert_eq!(scopes, vec![APPDATA_SCOPE.to_owned()]);
    assert!(
        scopes
            .iter()
            .all(|scope| scope == APPDATA_SCOPE || !scope.contains("drive")),
        "unexpected drive scopes: {scopes:?}"
    );
    Ok(())
}

// 001 «Новый пользователь»
#[test]
fn new_user_lands_on_main_with_starter_categories_only() -> Result<(), SessionError> {
    let mut session = Session::new(MemoryIdentity::new(), MemoryDrive::new());
    let place = sign_in_to_main(&mut session)?;
    assert_eq!(place, Place::Main);
    assert_eq!(session.place(), Place::Main);
    let ledger = session.ledger().expect("ledger after sign-in");
    assert_categories_match_starter(ledger);
    assert!(ledger.accounts().is_empty());
    assert!(ledger.operations().is_empty());
    Ok(())
}

// 001 «Возвращающийся пользователь»
#[test]
fn another_device_sees_accounts_categories_and_operations_after_sign_in() -> Result<(), SessionError>
{
    let drive = MemoryDrive::new();
    let phone_identity = MemoryIdentity::new();
    let mut phone = Session::new(phone_identity, drive.clone());
    sign_in_to_main(&mut phone)?;
    let day = date(2026, 11, 20).expect("valid date");
    pollster::block_on(phone.change(|ledger| {
        let account = create_account(ledger, "Карта", "RUB", 5_000)?;
        let food = create_top(ledger, "Поездки", Applicability::Expense)?;
        ledger.add_expense(NewExpense {
            date: day,
            account_id: Some(account),
            category_id: Some(food),
            amount: Some(350),
            comment: "Такси".into(),
        })?;
        Ok(())
    }))?;

    let laptop_identity = MemoryIdentity::new();
    let mut laptop = Session::new(laptop_identity, drive);
    sign_in_to_main(&mut laptop)?;
    assert_eq!(laptop.place(), Place::Main);
    let ledger = laptop.ledger().expect("ledger on second device");
    assert_eq!(ledger.accounts().len(), 1);
    assert_eq!(ledger.accounts()[0].name(), "Карта");
    assert!(ledger.categories().iter().any(|c| c.name() == "Поездки"));
    assert_eq!(ledger.operations().len(), 1);
    Ok(())
}

// 001 «Сохранение изменений»
#[test]
fn edits_persist_after_sign_out_and_sign_in_again() -> Result<(), SessionError> {
    let identity = MemoryIdentity::new();
    let drive = MemoryDrive::new();
    let mut session = Session::new(identity.clone(), drive.clone());
    sign_in_to_main(&mut session)?;

    let mut account_id = None;
    pollster::block_on(session.change(|ledger| {
        account_id = Some(create_account(ledger, "Кошелёк", "RUB", 1_000)?);
        Ok(())
    }))?;
    let account_id = account_id.expect("account id");

    pollster::block_on(session.change(|ledger| {
        ledger.update_account(
            account_id,
            UpdateAccount {
                name: Some("Наличные".into()),
                currency: None,
                credit_limit: None,
                note: None,
                opening_balance: None,
            },
        )?;
        Ok(())
    }))?;

    let day = date(2026, 11, 21).expect("valid date");
    let food = require_child_category(session.ledger().expect("ledger"), "Продукты");
    let mut expense_id = None;
    pollster::block_on(session.change(|ledger| {
        ledger.add_expense(NewExpense {
            date: day,
            account_id: Some(account_id),
            category_id: Some(food),
            amount: Some(120),
            comment: "Временный".into(),
        })?;
        expense_id = ledger
            .operations()
            .iter()
            .find(|operation| operation.kind() == OperationKind::Expense)
            .map(OperationEntry::id);
        Ok(())
    }))?;
    let expense_id = expense_id.expect("expense id");

    pollster::block_on(session.change(|ledger| {
        ledger.delete_operation(expense_id)?;
        Ok(())
    }))?;

    session.sign_out();
    let mut session = Session::new(identity, drive);
    sign_in_to_main(&mut session)?;
    let ledger = session.ledger().expect("ledger after return");
    assert_eq!(ledger.accounts().len(), 1);
    assert_eq!(ledger.accounts()[0].name(), "Наличные");
    assert!(ledger.operations().is_empty());
    Ok(())
}

// 001 «Изменения с другого устройства»
#[test]
fn on_return_reloads_operations_written_from_another_device() -> Result<(), SessionError> {
    let drive = MemoryDrive::new();
    let computer_identity = MemoryIdentity::new();
    let mut computer = Session::new(computer_identity, drive.clone());
    sign_in_to_main(&mut computer)?;

    let phone_identity = MemoryIdentity::new();
    let mut phone = Session::new(phone_identity, drive);
    sign_in_to_main(&mut phone)?;
    let day = date(2026, 11, 22).expect("valid date");
    pollster::block_on(phone.change(|ledger| {
        let account = create_account(ledger, "Наличные", "RUB", 0)?;
        let food = require_child_category(ledger, "Продукты");
        ledger.add_expense(NewExpense {
            date: day,
            account_id: Some(account),
            category_id: Some(food),
            amount: Some(90),
            comment: "phone-feed".into(),
        })?;
        Ok(())
    }))?;

    let place = pollster::block_on(computer.on_return())?;
    assert_eq!(place, Place::Main);
    assert_eq!(computer.place(), Place::Main);
    let ledger = computer.ledger().expect("computer ledger");
    assert_eq!(operation_comments(ledger), vec!["phone-feed".to_owned()]);
    Ok(())
}

// 001 «Сохранение не затирает изменения с другого устройства»
#[test]
fn change_keeps_operations_added_on_another_device() -> Result<(), SessionError> {
    let drive = MemoryDrive::new();
    let phone_identity = MemoryIdentity::new();
    let computer_identity = MemoryIdentity::new();
    let mut phone = Session::new(phone_identity, drive.clone());
    let mut computer = Session::new(computer_identity, drive);
    sign_in_to_main(&mut phone)?;
    sign_in_to_main(&mut computer)?;

    let day = date(2026, 11, 22).expect("valid date");
    pollster::block_on(phone.change(|ledger| {
        let account = create_account(ledger, "Наличные", "RUB", 0)?;
        let food = require_child_category(ledger, "Продукты");
        ledger.add_expense(NewExpense {
            date: day,
            account_id: Some(account),
            category_id: Some(food),
            amount: Some(50),
            comment: "from-phone".into(),
        })?;
        Ok(())
    }))?;

    pollster::block_on(computer.change(|ledger| {
        let account = create_account(ledger, "Карта", "RUB", 0)?;
        let food = require_child_category(ledger, "Продукты");
        ledger.add_expense(NewExpense {
            date: day,
            account_id: Some(account),
            category_id: Some(food),
            amount: Some(70),
            comment: "from-computer".into(),
        })?;
        Ok(())
    }))?;

    let ledger = computer.ledger().expect("computer ledger");
    let mut comments = operation_comments(ledger);
    comments.sort();
    assert_eq!(
        comments,
        vec!["from-computer".to_owned(), "from-phone".to_owned()]
    );
    Ok(())
}

// 001 «Отказ от входа»
#[test]
fn rejected_sign_in_stays_on_sign_in_with_required_message() {
    #[rustfmt::skip]
    let outcomes = [
        AuthError::Cancelled,
        AuthError::Denied,
    ];

    for auth_error in outcomes {
        let identity = MemoryIdentity::new();
        let mut session = Session::new(identity.clone(), MemoryDrive::new());
        identity.reject_next_sign_in(auth_error);
        pollster::block_on(session.open());
        let err = pollster::block_on(session.sign_in()).expect_err("sign-in must fail");
        assert_eq!(err, SessionError::Auth(auth_error));
        assert_eq!(
            session.place(),
            Place::SignIn {
                message: Some(SIGN_IN_REQUIRED),
            }
        );
        assert!(session.ledger().is_none());
    }
}

// 001 «Сеанс сохраняется»
#[test]
fn remembered_session_opens_to_confirm_then_main_without_new_sign_in() -> Result<(), SessionError> {
    let identity = MemoryIdentity::new();
    let drive = MemoryDrive::new();
    let mut session = Session::new(identity.clone(), drive.clone());
    sign_in_to_main(&mut session)?;
    pollster::block_on(session.change(|ledger| {
        create_account(ledger, "Наличные", "RUB", 500)?;
        Ok(())
    }))?;
    let sign_ins = identity.sign_in_count();
    let scopes = identity.requested_scopes();

    identity.expire_access();
    let mut session = Session::new(identity.clone(), drive);
    let place = pollster::block_on(session.open());
    assert_eq!(place, Place::Confirm);
    assert_eq!(session.place(), Place::Confirm);
    assert_eq!(identity.sign_in_count(), sign_ins);
    assert_eq!(identity.requested_scopes(), scopes);

    let place = pollster::block_on(session.confirm())?;
    assert_eq!(place, Place::Main);
    assert_eq!(session.place(), Place::Main);
    assert_eq!(identity.sign_in_count(), sign_ins);
    let ledger = session.ledger().expect("ledger after confirm");
    assert_eq!(ledger.accounts().len(), 1);
    assert_eq!(ledger.accounts()[0].name(), "Наличные");
    Ok(())
}

// 001 «Сеанс в долгой вкладке»
#[test]
fn change_after_expired_access_needs_confirm_then_keeps_pending_expense() -> Result<(), SessionError>
{
    let identity = MemoryIdentity::new();
    let mut session = Session::new(identity.clone(), MemoryDrive::new());
    sign_in_to_main(&mut session)?;

    let mut account_id = None;
    pollster::block_on(session.change(|ledger| {
        account_id = Some(create_account(ledger, "Наличные", "RUB", 3_000)?);
        Ok(())
    }))?;
    let account_id = account_id.expect("account id");
    let food = require_child_category(session.ledger().expect("ledger"), "Продукты");
    let day = date(2026, 11, 23).expect("valid date");
    let pending_expense = NewExpense {
        date: day,
        account_id: Some(account_id),
        category_id: Some(food),
        amount: Some(250),
        comment: "Долгая вкладка".into(),
    };

    let sign_ins = identity.sign_in_count();
    identity.expire_access();
    let err =
        pollster::block_on(session.change(|ledger| ledger.add_expense(pending_expense.clone())))
            .expect_err("change must ask for confirmation");
    assert_eq!(err, SessionError::Auth(AuthError::NeedsConfirmation));
    assert_eq!(session.place(), Place::Main);
    assert!(session.ledger().expect("ledger").operations().is_empty());
    assert_eq!(identity.sign_in_count(), sign_ins);

    pollster::block_on(session.confirm())?;
    assert_eq!(identity.sign_in_count(), sign_ins);

    pollster::block_on(session.change(|ledger| ledger.add_expense(pending_expense)))?;
    let ledger = session.ledger().expect("ledger");
    assert_eq!(ledger.operations().len(), 1);
    Ok(())
}

// 001 «Выход»
#[test]
fn sign_out_clears_ledger_and_next_open_shows_sign_in() -> Result<(), SessionError> {
    let identity = MemoryIdentity::new();
    let drive = MemoryDrive::new();
    let mut session = Session::new(identity.clone(), drive.clone());
    sign_in_to_main(&mut session)?;
    pollster::block_on(session.change(|ledger| {
        create_account(ledger, "Наличные", "RUB", 0)?;
        Ok(())
    }))?;

    let place = session.sign_out();
    assert_eq!(place, Place::SignIn { message: None });
    assert!(session.ledger().is_none());

    let mut session = Session::new(identity, drive);
    let place = pollster::block_on(session.open());
    assert_eq!(place, Place::SignIn { message: None });
    assert!(session.ledger().is_none());
    Ok(())
}

// 001 «Выход не затрагивает другие устройства и Google»
#[test]
fn phone_sign_out_leaves_computer_signed_in_and_google_account_active() -> Result<(), SessionError>
{
    let drive = MemoryDrive::new();
    let phone_identity = MemoryIdentity::new();
    let computer_identity = MemoryIdentity::new();
    let mut phone = Session::new(phone_identity.clone(), drive.clone());
    let mut computer = Session::new(computer_identity, drive);
    sign_in_to_main(&mut phone)?;
    sign_in_to_main(&mut computer)?;
    pollster::block_on(computer.change(|ledger| {
        create_account(ledger, "Зарплатная", "RUB", 10_000)?;
        Ok(())
    }))?;

    phone.sign_out();
    assert_eq!(phone.place(), Place::SignIn { message: None });
    assert!(phone.ledger().is_none());
    assert!(phone_identity.google_account_signed_in());

    assert_eq!(computer.place(), Place::Main);
    let ledger = computer.ledger().expect("computer ledger");
    assert_eq!(ledger.accounts().len(), 1);
    assert_eq!(ledger.accounts()[0].name(), "Зарплатная");
    Ok(())
}
