mod draft;
mod form;
mod mode;
mod quick;
mod text;

use leptos::prelude::*;

use domain::{FeedDay, FeedLine, Ledger, OperationId, OperationKind};

use draft::initial_ledger;
use form::Editor;
use mode::Mode;
use quick::quick_page;
use text::{format_day, kind_name, line_text};

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let ledger = RwSignal::new(initial_ledger());
    let mode = RwSignal::new(Mode::Feed);
    view! {
        <main>
            <h1>"Покупкомер"</h1>
            {move || match mode.get() {
                Mode::Feed => feed_page(ledger, mode).into_any(),
                Mode::Create(kind) => create_page(ledger, mode, kind).into_any(),
                Mode::Edit(id) => edit_page(ledger, mode, id).into_any(),
                Mode::Quick => quick_page(ledger, mode).into_any(),
            }}
        </main>
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

fn create_page(
    ledger: RwSignal<Ledger>,
    mode: RwSignal<Mode>,
    kind: OperationKind,
) -> impl IntoView {
    let editing: Option<OperationId> = None;
    view! { <Editor ledger mode editing initial_kind=kind /> }
}

fn edit_page(ledger: RwSignal<Ledger>, mode: RwSignal<Mode>, id: OperationId) -> impl IntoView {
    let kind = ledger.with_untracked(|ledger| {
        ledger
            .operation_form(id)
            .map(|form| form.kind())
            .unwrap_or(OperationKind::Expense)
    });
    view! { <Editor ledger mode editing=Some(id) initial_kind=kind /> }
}
