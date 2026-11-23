use std::cell::RefCell;
use std::rc::Rc;

use futures::channel::oneshot;
use js_sys::{Function, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::window;

use domain::AuthError;

pub(crate) struct Granted {
    pub(crate) access_token: String,
    pub(crate) expires_in: f64,
}

type Reply = oneshot::Sender<Result<Granted, AuthError>>;
type ReplySlot = Rc<RefCell<Option<Reply>>>;

struct Held {
    _client: JsValue,
    _callback: Closure<dyn FnMut(JsValue)>,
    _error: Closure<dyn FnMut(JsValue)>,
}

thread_local! {
    static HELD: RefCell<Option<Held>> = const { RefCell::new(None) };
}

pub(crate) async fn request_access_token(
    client_id: &str,
    scope: &str,
) -> Result<Granted, AuthError> {
    let (sender, receiver) = oneshot::channel();
    start_request(client_id, scope, sender)?;
    let granted = receiver.await.map_err(|_| AuthError::Cancelled)?;
    release();
    granted
}

fn start_request(client_id: &str, scope: &str, sender: Reply) -> Result<(), AuthError> {
    let sender = Rc::new(RefCell::new(Some(sender)));
    let callback = token_callback(sender.clone());
    let error_callback = failure_callback(sender);
    let client = token_client(client_id, scope, &callback, &error_callback)?;
    request_token(&client)?;
    hold(client, callback, error_callback);
    Ok(())
}

fn token_callback(sender: ReplySlot) -> Closure<dyn FnMut(JsValue)> {
    Closure::new(move |value: JsValue| {
        reply(&sender, granted_from(&value));
    })
}

fn failure_callback(sender: ReplySlot) -> Closure<dyn FnMut(JsValue)> {
    Closure::new(move |value: JsValue| {
        reply(&sender, Err(failure_from(&value)));
    })
}

fn reply(sender: &RefCell<Option<Reply>>, result: Result<Granted, AuthError>) {
    let Some(sender) = sender.borrow_mut().take() else {
        return;
    };
    let _ = sender.send(result);
}

fn token_client(
    client_id: &str,
    scope: &str,
    callback: &Closure<dyn FnMut(JsValue)>,
    error_callback: &Closure<dyn FnMut(JsValue)>,
) -> Result<JsValue, AuthError> {
    let init = method(&oauth2()?, "initTokenClient")?;
    let config = Object::new();
    set_str(&config, "client_id", client_id)?;
    set_str(&config, "scope", scope)?;
    set_value(&config, "callback", callback.as_ref().unchecked_ref())?;
    set_value(
        &config,
        "error_callback",
        error_callback.as_ref().unchecked_ref(),
    )?;
    init.call1(&JsValue::NULL, &config)
        .map_err(|_| AuthError::Cancelled)
}

fn request_token(client: &JsValue) -> Result<(), AuthError> {
    let method = method(client, "requestAccessToken")?;
    method
        .call0(client)
        .map(|_| ())
        .map_err(|_| AuthError::Cancelled)
}

fn oauth2() -> Result<JsValue, AuthError> {
    let window = window().ok_or(AuthError::Cancelled)?;
    let google = Reflect::get(&window, &JsValue::from_str("google")).map_err(cancelled)?;
    let accounts = Reflect::get(&google, &JsValue::from_str("accounts")).map_err(cancelled)?;
    Reflect::get(&accounts, &JsValue::from_str("oauth2")).map_err(cancelled)
}

fn method(value: &JsValue, name: &str) -> Result<Function, AuthError> {
    Reflect::get(value, &JsValue::from_str(name))
        .map_err(cancelled)?
        .dyn_into()
        .map_err(cancelled)
}

fn set_str(object: &Object, name: &str, value: &str) -> Result<(), AuthError> {
    set_value(object, name, &JsValue::from_str(value))
}

fn set_value(object: &Object, name: &str, value: &JsValue) -> Result<(), AuthError> {
    Reflect::set(object, &JsValue::from_str(name), value).map_err(cancelled)?;
    Ok(())
}

fn granted_from(value: &JsValue) -> Result<Granted, AuthError> {
    if let Some(error) = text_field(value, "error") {
        return Err(named_error(&error));
    }
    Ok(Granted {
        access_token: text_field(value, "access_token").ok_or(AuthError::Cancelled)?,
        expires_in: number_field(value, "expires_in").unwrap_or(3600.0),
    })
}

fn failure_from(value: &JsValue) -> AuthError {
    text_field(value, "type")
        .or_else(|| text_field(value, "error"))
        .as_deref()
        .map(named_error)
        .unwrap_or(AuthError::Cancelled)
}

fn named_error(name: &str) -> AuthError {
    if name == "access_denied" {
        AuthError::Denied
    } else {
        AuthError::Cancelled
    }
}

fn text_field(value: &JsValue, name: &str) -> Option<String> {
    Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|field| field.as_string())
}

fn number_field(value: &JsValue, name: &str) -> Option<f64> {
    Reflect::get(value, &JsValue::from_str(name))
        .ok()
        .and_then(|field| field.as_f64())
}

fn cancelled(_: JsValue) -> AuthError {
    AuthError::Cancelled
}

fn hold(
    client: JsValue,
    callback: Closure<dyn FnMut(JsValue)>,
    error: Closure<dyn FnMut(JsValue)>,
) {
    HELD.with(|slot| {
        *slot.borrow_mut() = Some(Held {
            _client: client,
            _callback: callback,
            _error: error,
        });
    });
}

fn release() {
    HELD.with(|slot| {
        slot.borrow_mut().take();
    });
}
