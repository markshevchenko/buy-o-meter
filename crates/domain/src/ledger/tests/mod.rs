mod account;
mod category;
mod debt;
mod posting;
mod removal;
mod transfer;

pub(super) fn omit_if<T>(missing: bool, value: T) -> Option<T> {
    if missing { None } else { Some(value) }
}
