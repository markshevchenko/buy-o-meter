use leptos::prelude::*;

use domain::{Applicability, Category, CategoryId, CreateCategory, Ledger, UpdateCategory};

use crate::api::AppApi;

pub(crate) fn categories_page(api: AppApi) -> impl IntoView {
    let ledger = api.ledger();
    let editing = RwSignal::new(None::<CategoryId>);
    let list = api.clone();
    view! {
        <div class="tree">
            {move || {
                let top: Vec<Category> = ledger
                    .with(|ledger| {
                        ledger
                            .categories()
                            .iter()
                            .filter(|category| category.parent_id().is_none())
                            .cloned()
                            .collect()
                    });
                top.into_iter()
                    .map(|category| category_group(list.clone(), editing, category))
                    .collect_view()
            }}
        </div>
        {add_form(api)}
    }
}

fn category_group(
    api: AppApi,
    editing: RwSignal<Option<CategoryId>>,
    category: Category,
) -> impl IntoView {
    let ledger = api.ledger();
    let parent_id = category.id();
    view! {
        {category_row(api.clone(), editing, category)}
        <div class="sub">
            {move || {
                let subs: Vec<Category> = ledger
                    .with(|ledger| {
                        ledger
                            .categories()
                            .iter()
                            .filter(|category| category.parent_id() == Some(parent_id))
                            .cloned()
                            .collect()
                    });
                subs.into_iter()
                    .map(|category| category_row(api.clone(), editing, category))
                    .collect_view()
            }}
        </div>
    }
}

fn category_row(
    api: AppApi,
    editing: RwSignal<Option<CategoryId>>,
    category: Category,
) -> impl IntoView {
    let id = category.id();
    let name = category.name().to_owned();
    view! {
        <button type="button" class="line" on:click=move |_| toggle(editing, id)>
            {name}
        </button>
        {move || {
            if editing.get() == Some(id) {
                edit_panel(api.clone(), id).into_any()
            } else {
                ().into_any()
            }
        }}
    }
}

fn toggle(editing: RwSignal<Option<CategoryId>>, id: CategoryId) {
    if editing.get() == Some(id) {
        editing.set(None);
    } else {
        editing.set(Some(id));
    }
}

fn edit_panel(api: AppApi, id: CategoryId) -> impl IntoView {
    let ledger = api.ledger();
    let Some((name, applicability, parent_id)) = ledger.with_untracked(|ledger| {
        ledger.category(id).ok().map(|category| {
            (
                category.name().to_owned(),
                category.applicability(),
                category.parent_id(),
            )
        })
    }) else {
        return ().into_any();
    };
    let name = RwSignal::new(name);
    let applicability = RwSignal::new(applicability);
    let new_parent = RwSignal::new(String::new());
    let message = RwSignal::new(None::<String>);
    let rename = api.clone();
    let move_api = api.clone();
    view! {
        <fieldset>
            <label>
                "Название"
                <input
                    type="text"
                    prop:value=move || name.get()
                    on:input=move |event| name.set(event_target_value(&event))
                />
            </label>
            {applicability_select(applicability)}
            <button
                type="button"
                on:click=move |_| {
                    rename.update_category(
                        id,
                        UpdateCategory {
                            name: Some(name.get()),
                            applicability: Some(applicability.get()),
                        },
                        message,
                    );
                }
            >
                "Сохранить"
            </button>
            {parent_id
                .map(|_| move_section(move_api.clone(), id, new_parent, message))
                .unwrap_or_else(|| ().into_any())}
            <p class="error">{move || message.get().unwrap_or_default()}</p>
            {delete_section(api.clone(), id, message)}
        </fieldset>
    }
    .into_any()
}

fn applicability_select(value: RwSignal<Applicability>) -> impl IntoView {
    let current = value.get();
    view! {
        <label>
            "Применимость"
            <select on:change=move |event| {
                if let Some(next) = applicability_from_str(&event_target_value(&event)) {
                    value.set(next);
                }
            }>
                <option value="expense" selected={current == Applicability::Expense}>"Расход"</option>
                <option value="income" selected={current == Applicability::Income}>"Доход"</option>
                <option value="both" selected={current == Applicability::Both}>"Расход и доход"</option>
            </select>
        </label>
    }
}

fn applicability_from_str(value: &str) -> Option<Applicability> {
    match value {
        "expense" => Some(Applicability::Expense),
        "income" => Some(Applicability::Income),
        "both" => Some(Applicability::Both),
        _ => None,
    }
}

fn move_section(
    api: AppApi,
    id: CategoryId,
    new_parent: RwSignal<String>,
    message: RwSignal<Option<String>>,
) -> AnyView {
    let ledger = api.ledger();
    view! {
        <label class="quiet">
            "Перенести к другой категории верхнего уровня"
            {move || {
                let options: Vec<Category> = ledger
                    .with(|ledger| {
                        ledger
                            .categories()
                            .iter()
                            .filter(|category| category.parent_id().is_none() && category.id() != id)
                            .cloned()
                            .collect()
                    });
                view! {
                    <select on:change=move |event| new_parent.set(event_target_value(&event))>
                        <option value="">"Выберите"</option>
                        {options
                            .into_iter()
                            .map(|category| {
                                let value = category.id().as_u64().to_string();
                                view! { <option value=value>{category.name().to_owned()}</option> }
                            })
                            .collect_view()}
                    </select>
                }
            }}
        </label>
        <button
            type="button"
            on:click=move |_| {
                let Ok(new_parent_id) = new_parent.get().parse::<u64>() else {
                    message.set(Some("Выберите категорию для переноса".to_owned()));
                    return;
                };
                api.move_category(id, CategoryId::from_u64(new_parent_id), message);
            }
        >
            "Перенести"
        </button>
    }
    .into_any()
}

fn delete_section(api: AppApi, id: CategoryId, message: RwSignal<Option<String>>) -> impl IntoView {
    let confirming = RwSignal::new(false);
    move || {
        if !confirming.get() {
            return view! {
                <button type="button" on:click=move |_| confirming.set(true)>"Удалить"</button>
            }
            .into_any();
        }
        delete_confirm(api.clone(), id, confirming, message)
    }
}

fn delete_confirm(
    api: AppApi,
    id: CategoryId,
    confirming: RwSignal<bool>,
    message: RwSignal<Option<String>>,
) -> AnyView {
    let ledger = api.ledger();
    let transfer_to = RwSignal::new(String::new());
    let targets: Vec<Category> = ledger.with_untracked(|ledger| {
        ledger
            .transfer_targets(id)
            .map(|targets| targets.into_iter().cloned().collect())
            .unwrap_or_default()
    });
    view! {
        <p>"Удалить эту категорию?"</p>
        {(!targets.is_empty())
            .then(|| {
                view! {
                    <label class="quiet">
                        "Перенести операции в"
                        <select on:change=move |event| transfer_to.set(event_target_value(&event))>
                            <option value="">"Выберите"</option>
                            {targets
                                .iter()
                                .map(|category| {
                                    let value = category.id().as_u64().to_string();
                                    view! { <option value=value>{category.name().to_owned()}</option> }
                                })
                                .collect_view()}
                        </select>
                    </label>
                }
            })}
        <button
            type="button"
            on:click=move |_| {
                let target = transfer_to.get().parse::<u64>().ok().map(CategoryId::from_u64);
                api.delete_category(id, target, message);
            }
        >
            "Удалить"
        </button>
        <button type="button" on:click=move |_| confirming.set(false)>"Отмена"</button>
    }
    .into_any()
}

fn parent_select(ledger: RwSignal<Ledger>, parent: RwSignal<String>) -> impl IntoView {
    move || {
        let options: Vec<Category> = ledger.with(|ledger| {
            ledger
                .categories()
                .iter()
                .filter(|category| category.parent_id().is_none())
                .cloned()
                .collect()
        });
        view! {
            <select on:change=move |event| parent.set(event_target_value(&event))>
                <option value="">"Без родителя"</option>
                {options
                    .into_iter()
                    .map(|category| {
                        let value = category.id().as_u64().to_string();
                        view! { <option value=value>{category.name().to_owned()}</option> }
                    })
                    .collect_view()}
            </select>
        }
    }
}

fn new_applicability_field(parent: RwSignal<String>, applicability: RwSignal<String>) -> AnyView {
    if !parent.get().is_empty() {
        return ().into_any();
    }
    view! {
        <label>
            "Применимость"
            <select on:change=move |event| applicability.set(event_target_value(&event))>
                <option value="">"Выберите"</option>
                <option value="expense">"Расход"</option>
                <option value="income">"Доход"</option>
                <option value="both">"Расход и доход"</option>
            </select>
        </label>
    }
    .into_any()
}

fn add_form(api: AppApi) -> impl IntoView {
    let ledger = api.ledger();
    let name = RwSignal::new(String::new());
    let parent = RwSignal::new(String::new());
    let applicability = RwSignal::new(String::new());
    let message = RwSignal::new(None::<String>);
    view! {
        <fieldset>
            <legend>"Новая категория"</legend>
            <label>
                "Название"
                <input
                    type="text"
                    prop:value=move || name.get()
                    on:input=move |event| name.set(event_target_value(&event))
                />
            </label>
            <label>"Родитель" {parent_select(ledger, parent)}</label>
            {move || new_applicability_field(parent, applicability)}
            <button
                type="button"
                on:click=move |_| add_category(api.clone(), name, parent, applicability, message)
            >
                "Добавить"
            </button>
            <p class="error">{move || message.get().unwrap_or_default()}</p>
        </fieldset>
    }
}

fn add_category(
    api: AppApi,
    name: RwSignal<String>,
    parent: RwSignal<String>,
    applicability: RwSignal<String>,
    message: RwSignal<Option<String>>,
) {
    let category_name = name.get().trim().to_owned();
    if category_name.is_empty() {
        message.set(Some("Укажите название".to_owned()));
        return;
    }
    let parent_id = parent.get().parse::<u64>().ok().map(CategoryId::from_u64);
    let applicability = applicability_from_str(&applicability.get());
    api.create_category(
        CreateCategory {
            name: category_name,
            parent_id,
            applicability,
        },
        message,
    );
    name.set(String::new());
}
