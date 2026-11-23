use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response, window};

use domain::DriveError;

pub(crate) async fn send(
    method: &str,
    url: &str,
    token: &str,
    content_type: Option<&str>,
    body: Option<Vec<u8>>,
) -> Result<Vec<u8>, DriveError> {
    let request = build(method, url, token, content_type, body)?;
    let response = dispatch(&request).await?;
    read_body(response).await
}

fn build(
    method: &str,
    url: &str,
    token: &str,
    content_type: Option<&str>,
    body: Option<Vec<u8>>,
) -> Result<Request, DriveError> {
    let init = RequestInit::new();
    init.set_method(method);
    let headers = web_sys::Headers::new().map_err(unavailable)?;
    headers
        .set("Authorization", &format!("Bearer {token}"))
        .map_err(unavailable)?;
    if let Some(content_type) = content_type {
        headers
            .set("Content-Type", content_type)
            .map_err(unavailable)?;
    }
    init.set_headers(headers.as_ref());
    if let Some(body) = body {
        let array = Uint8Array::from(body.as_slice());
        init.set_body(array.as_ref());
    }
    Request::new_with_str_and_init(url, &init).map_err(unavailable)
}

async fn dispatch(request: &Request) -> Result<Response, DriveError> {
    let window = window().ok_or(DriveError::Unavailable)?;
    let value = JsFuture::from(window.fetch_with_request(request))
        .await
        .map_err(unavailable)?;
    value.dyn_into().map_err(unavailable)
}

async fn read_body(response: Response) -> Result<Vec<u8>, DriveError> {
    if !response.ok() {
        return Err(DriveError::Unavailable);
    }
    let promise = response.array_buffer().map_err(unavailable)?;
    let buffer = JsFuture::from(promise).await.map_err(unavailable)?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

fn unavailable(_: wasm_bindgen::JsValue) -> DriveError {
    DriveError::Unavailable
}
