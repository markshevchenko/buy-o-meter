mod accounts;
mod api;
mod categories;
mod draft;
mod drive_api;
mod form;
mod gis;
mod google;
mod http;
mod mode;
mod quick;
mod tab;
mod text;
mod token;

use leptos::prelude::*;

use domain::{FeedDay, FeedLine, Ledger, OperationId, OperationKind, Place};

use accounts::accounts_page;
use api::AppApi;
use categories::categories_page;
use form::Editor;
use google::client_id;
use mode::Mode;
use quick::quick_page;
use tab::Tab;
use text::{format_day, kind_name, line_text};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let api = AppApi::new();
    api.open();
    api.watch_visibility();
    let nav_place = api.clone();
    let nav = api.clone();
    let screen = api.clone();
    let body = api.clone();
    view! {
        {move || {
            if nav_place.place().get() == Place::Main {
                nav_bar(nav.clone()).into_any()
            } else {
                ().into_any()
            }
        }}
        <main>
            {move || match body.place().get() {
                Place::SignIn { message } => sign_in_screen(screen.clone(), message).into_any(),
                Place::Confirm => confirm_screen(screen.clone()).into_any(),
                Place::Main => main_screen(screen.clone()).into_any(),
            }}
        </main>
    }
}

fn nav_bar(api: AppApi) -> impl IntoView {
    let tab = api.tab();
    let logo = api.clone();
    view! {
        <nav class="nav">
            <button type="button" class="logo" on:click=move |_| logo.tab().set(Tab::About)>
                <img src="/assets/logo.svg" alt="Покупкомер" />
            </button>
            <div class="tabs">
                {Tab::ALL.into_iter().map(|item| tab_button(tab, item)).collect_view()}
            </div>
        </nav>
    }
}

fn tab_button(tab: RwSignal<Tab>, item: Tab) -> impl IntoView {
    view! {
        <button
            type="button"
            class:active=move || tab.get() == item
            on:click=move |_| tab.set(item)
        >
            {item.name()}
        </button>
    }
}

fn sign_in_screen(api: AppApi, message: Option<&'static str>) -> impl IntoView {
    view! {
        <section class="auth">
            <img class="brand" src="/assets/logo.svg" alt="" />
            {about_app()}
            {message.map(|text| view! { <p class="error">{text}</p> })}
            {sign_in_action(api)}
        </section>
    }
}

fn about_app() -> impl IntoView {
    view! {
        <p>
            "Приложение " <strong>"Покупкомер"</strong> " — учёт личных финансов. "
            "Данные хранятся на вашем Google Drive, поэтому для работы нужен "
            "Google-аккаунт. У приложения не будет доступа к вашим личным данным, "
            "документам и паролю, только к личным финансам, которые вы вводите."
        </p>
    }
}

fn sign_in_action(api: AppApi) -> AnyView {
    if client_id().is_none() {
        return view! { <p class="error">"Вход не настроен"</p> }.into_any();
    }
    view! { <button type="button" on:click=move |_| api.sign_in()>"Войти"</button> }.into_any()
}

fn confirm_screen(api: AppApi) -> impl IntoView {
    let error = api.confirm_error();
    view! {
        <section class="auth">
            <p>"Подтвердите вход через Google."</p>
            {confirm_action(api)}
            <p class="error">{move || error.get().unwrap_or_default()}</p>
        </section>
    }
}

fn confirm_action(api: AppApi) -> AnyView {
    if client_id().is_none() {
        return view! { <p class="error">"Вход не настроен"</p> }.into_any();
    }
    view! { <button type="button" on:click=move |_| api.confirm()>"Подтвердить"</button> }
        .into_any()
}

fn main_screen(api: AppApi) -> impl IntoView {
    let tab = api.tab();
    let banner = api.clone();
    let operations = api.clone();
    let categories = api.clone();
    let accounts = api.clone();
    let about = api.clone();
    view! {
        {confirm_banner(banner)}
        {move || match tab.get() {
            Tab::Operations => operations_page(operations.clone()).into_any(),
            Tab::Reports => reports_page().into_any(),
            Tab::Categories => categories_page(categories.clone()).into_any(),
            Tab::Accounts => accounts_page(accounts.clone()).into_any(),
            Tab::About => about_page(about.clone()).into_any(),
        }}
    }
}

fn operations_page(api: AppApi) -> impl IntoView {
    let mode = api.mode();
    let ledger = api.ledger();
    let pages = api.clone();
    view! {
        {move || match mode.get() {
            Mode::Feed => feed_page(ledger, mode).into_any(),
            Mode::Create(kind) => create_page(pages.clone(), kind).into_any(),
            Mode::Edit(id) => edit_page(pages.clone(), id).into_any(),
            Mode::Quick => quick_page(ledger, mode).into_any(),
        }}
    }
}

fn reports_page() -> impl IntoView {
    view! { <p>"Скоро"</p> }
}

fn about_page(api: AppApi) -> impl IntoView {
    view! {
        <h2>"Покупкомер"</h2>
        <button type="button" on:click=move |_| api.sign_out()>"Выйти"</button>
    }
}

fn confirm_banner(api: AppApi) -> impl IntoView {
    let shown = api.clone();
    view! {
        {move || {
            if !shown.needs_confirm().get() {
                return ().into_any();
            }
            let error = shown.confirm_error();
            let api = shown.clone();
            view! {
                <section class="banner">
                    <p>"Подтвердите вход через Google."</p>
                    <button type="button" on:click=move |_| api.confirm()>"Подтвердить"</button>
                    <p class="error">{move || error.get().unwrap_or_default()}</p>
                </section>
            }
            .into_any()
        }}
    }
}

fn feed_page(ledger: RwSignal<Ledger>, mode: RwSignal<Mode>) -> impl IntoView {
    view! {
        {move || feed_body(ledger, mode)}
        <section class="add">
            <button type="button" class="link" on:click=move |_| mode.set(Mode::Quick)>
                "Остатки и долги"
            </button>
            <p>"Добавить"</p>
            {add_buttons(ledger, mode)}
        </section>
    }
}

fn add_buttons(ledger: RwSignal<Ledger>, mode: RwSignal<Mode>) -> impl IntoView {
    move || {
        let kinds = ledger.with(|ledger| ledger.main_screen().add_kinds().to_vec());
        view! {
            <div class="kinds">
                {kinds
                    .into_iter()
                    .map(|kind| kind_button(kind, mode))
                    .collect_view()}
            </div>
        }
    }
}

fn kind_button(kind: OperationKind, mode: RwSignal<Mode>) -> impl IntoView {
    view! {
        <button type="button" on:click=move |_| mode.set(Mode::Create(kind))>
            {kind_name(kind)}
        </button>
    }
}

fn feed_body(ledger: RwSignal<Ledger>, mode: RwSignal<Mode>) -> AnyView {
    let screen = ledger.with(Ledger::main_screen);
    if screen.is_empty() {
        return view! { <p class="invite">"Добавьте первую операцию"</p> }.into_any();
    }
    screen
        .days()
        .iter()
        .cloned()
        .map(|day| day_section(day, mode))
        .collect_view()
        .into_any()
}

fn day_section(day: FeedDay, mode: RwSignal<Mode>) -> impl IntoView {
    let heading = format_day(day.date());
    let lines = day.lines().to_vec();
    view! {
        <section class="day">
            <h2>{heading}</h2>
            {lines
                .into_iter()
                .map(|line| line_button(line, mode))
                .collect_view()}
        </section>
    }
}

fn line_button(line: FeedLine, mode: RwSignal<Mode>) -> impl IntoView {
    let id = line.id();
    let text = line_text(&line);
    view! {
        <button type="button" class="line" on:click=move |_| mode.set(Mode::Edit(id))>
            {text}
        </button>
    }
}

fn create_page(api: AppApi, kind: OperationKind) -> impl IntoView {
    let editing: Option<OperationId> = None;
    view! { <Editor api editing initial_kind=kind /> }
}

fn edit_page(api: AppApi, id: OperationId) -> impl IntoView {
    let kind = api.ledger().with_untracked(|ledger| {
        ledger
            .operation_form(id)
            .map(|form| form.kind())
            .unwrap_or(OperationKind::Expense)
    });
    view! { <Editor api editing=Some(id) initial_kind=kind /> }
}
