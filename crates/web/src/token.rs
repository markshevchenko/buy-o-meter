use std::sync::{Arc, Mutex};

const REMEMBERED_KEY: &str = "buy-o-meter-signed-in";

#[derive(Clone)]
pub(crate) struct Tokens {
    inner: Arc<Mutex<TokenState>>,
}

struct TokenState {
    access_token: Option<String>,
    expires_at: f64,
    remembered: bool,
    prompt_ready: bool,
}

impl Tokens {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(TokenState {
                access_token: None,
                expires_at: 0.0,
                remembered: read_flag(),
                prompt_ready: false,
            })),
        }
    }

    pub(crate) fn remembered(&self) -> bool {
        self.with(false, |state| state.remembered)
    }

    pub(crate) fn current(&self) -> Option<String> {
        self.with(None, |state| {
            if js_sys::Date::now() < state.expires_at {
                state.access_token.clone()
            } else {
                None
            }
        })
    }

    pub(crate) fn store(&self, access_token: String, expires_in: f64) {
        self.with((), |state| {
            state.access_token = Some(access_token);
            state.expires_at = js_sys::Date::now() + (expires_in * 1000.0 - 60_000.0).max(0.0);
            state.remembered = true;
            state.prompt_ready = false;
        });
        write_flag(true);
    }

    pub(crate) fn forget(&self) {
        self.with((), |state| {
            state.access_token = None;
            state.expires_at = 0.0;
            state.remembered = false;
            state.prompt_ready = false;
        });
        write_flag(false);
    }

    /// Первый вызов только взводит запрос. Следующий уже может открыть окно Google.
    pub(crate) fn begin_prompt(&self) -> bool {
        self.with(false, |state| {
            if state.prompt_ready {
                state.prompt_ready = false;
                true
            } else {
                state.prompt_ready = true;
                false
            }
        })
    }

    pub(crate) fn arm_prompt(&self) {
        self.with((), |state| state.prompt_ready = true);
    }

    fn with<T>(&self, fallback: T, body: impl FnOnce(&mut TokenState) -> T) -> T {
        self.inner
            .lock()
            .map(|mut state| body(&mut state))
            .unwrap_or(fallback)
    }
}

fn read_flag() -> bool {
    storage()
        .and_then(|storage| storage.get_item(REMEMBERED_KEY).ok().flatten())
        .is_some()
}

fn write_flag(value: bool) {
    let Some(storage) = storage() else {
        return;
    };
    if value {
        let _ = storage.set_item(REMEMBERED_KEY, "1");
    } else {
        let _ = storage.remove_item(REMEMBERED_KEY);
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}
