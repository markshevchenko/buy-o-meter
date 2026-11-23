use std::fmt::{self, Debug, Formatter};

use serde::Deserialize;

use domain::{DATA_FILES, Drive, DriveError, DriveFile, FileWrite, Revision};

use crate::http;
use crate::token::Tokens;

const LIST_URL: &str = "https://www.googleapis.com/drive/v3/files?spaces=appDataFolder&fields=files(id,name,version)&pageSize=100";
const UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";

#[derive(Clone)]
pub(crate) struct GoogleDrive {
    tokens: Tokens,
}

impl GoogleDrive {
    pub(crate) fn new(tokens: Tokens) -> Self {
        Self { tokens }
    }
}

impl Debug for GoogleDrive {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("GoogleDrive")
    }
}

impl Drive for GoogleDrive {
    async fn load(&self) -> Result<Vec<DriveFile>, DriveError> {
        let token = self.tokens.current().ok_or(DriveError::Unavailable)?;
        let remote = list_files(&token).await?;
        let chosen = data_files(&remote);
        if chosen.is_empty() {
            return Ok(Vec::new());
        }
        if chosen.len() != DATA_FILES.len() {
            return Err(DriveError::Unavailable);
        }
        download_all(&token, &chosen).await
    }

    async fn write(&mut self, files: Vec<FileWrite>) -> Result<(), DriveError> {
        let token = self.tokens.current().ok_or(DriveError::Unavailable)?;
        let remote = list_files(&token).await?;
        if files.iter().any(|file| conflicts(file, &remote)) {
            return Err(DriveError::Conflict);
        }
        commit(&token, &files, &remote).await
    }
}

#[derive(Clone)]
struct RemoteFile {
    id: String,
    name: String,
    revision: Revision,
}

#[derive(Deserialize)]
struct Listed {
    files: Option<Vec<ListedFile>>,
}

#[derive(Deserialize)]
struct ListedFile {
    id: String,
    name: String,
    #[serde(default)]
    version: VersionField,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum VersionField {
    Text(String),
    Number(u64),
}

impl Default for VersionField {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

async fn list_files(token: &str) -> Result<Vec<RemoteFile>, DriveError> {
    let bytes = http::send("GET", LIST_URL, token, None, None).await?;
    let listed: Listed = serde_json::from_slice(&bytes).map_err(|_| DriveError::Unavailable)?;
    listed
        .files
        .unwrap_or_default()
        .into_iter()
        .map(remote_file)
        .collect()
}

fn remote_file(file: ListedFile) -> Result<RemoteFile, DriveError> {
    Ok(RemoteFile {
        id: file.id,
        name: file.name,
        revision: revision(&file.version)?,
    })
}

fn revision(version: &VersionField) -> Result<Revision, DriveError> {
    let text = match version {
        VersionField::Text(text) if !text.is_empty() => text.clone(),
        VersionField::Number(number) => number.to_string(),
        VersionField::Text(_) => return Err(DriveError::Unavailable),
    };
    text.parse()
        .map(Revision::from_u64)
        .map_err(|_| DriveError::Unavailable)
}

async fn download_all(token: &str, files: &[RemoteFile]) -> Result<Vec<DriveFile>, DriveError> {
    let mut loaded = Vec::new();
    for file in files {
        loaded.push(download(token, file).await?);
    }
    Ok(loaded)
}

fn data_files(remote: &[RemoteFile]) -> Vec<RemoteFile> {
    remote
        .iter()
        .filter(|file| DATA_FILES.contains(&file.name.as_str()))
        .cloned()
        .collect()
}

struct Backup {
    id: String,
    previous: Option<Vec<u8>>,
}

async fn commit(token: &str, files: &[FileWrite], remote: &[RemoteFile]) -> Result<(), DriveError> {
    let mut done = Vec::new();
    for file in files {
        if let Err(error) = store_one(token, file, remote, &mut done).await {
            undo(token, &done).await;
            return Err(error);
        }
    }
    Ok(())
}

async fn store_one(
    token: &str,
    file: &FileWrite,
    remote: &[RemoteFile],
    done: &mut Vec<Backup>,
) -> Result<(), DriveError> {
    let existing = remote.iter().find(|item| item.name == file.name);
    let previous = previous_bytes(token, existing).await?;
    let id = write_one(token, file, existing).await?;
    done.push(Backup { id, previous });
    Ok(())
}

async fn previous_bytes(
    token: &str,
    existing: Option<&RemoteFile>,
) -> Result<Option<Vec<u8>>, DriveError> {
    let Some(current) = existing else {
        return Ok(None);
    };
    let file = download(token, current).await?;
    Ok(Some(file.bytes().to_vec()))
}

async fn undo(token: &str, done: &[Backup]) {
    for backup in done.iter().rev() {
        let _ = restore(token, backup).await;
    }
}

async fn restore(token: &str, backup: &Backup) -> Result<(), DriveError> {
    match &backup.previous {
        Some(bytes) => patch(token, &backup.id, bytes).await,
        None => delete(token, &backup.id).await,
    }
}

async fn download(token: &str, file: &RemoteFile) -> Result<DriveFile, DriveError> {
    let url = format!(
        "https://www.googleapis.com/drive/v3/files/{}?alt=media",
        encode(&file.id)
    );
    let bytes = http::send("GET", &url, token, None, None).await?;
    Ok(DriveFile::new(file.name.clone(), file.revision, bytes))
}

async fn write_one(
    token: &str,
    file: &FileWrite,
    existing: Option<&RemoteFile>,
) -> Result<String, DriveError> {
    match existing {
        Some(current) => {
            patch(token, &current.id, &file.bytes).await?;
            Ok(current.id.clone())
        }
        None => create(token, &file.name, &file.bytes).await,
    }
}

async fn create(token: &str, name: &str, bytes: &[u8]) -> Result<String, DriveError> {
    let url = format!("{UPLOAD_URL}?uploadType=multipart&fields=id,version");
    let body = http::send(
        "POST",
        &url,
        token,
        Some("multipart/related; boundary=buyometer"),
        Some(multipart(name, bytes)),
    )
    .await?;
    let uploaded: Uploaded = serde_json::from_slice(&body).map_err(|_| DriveError::Unavailable)?;
    Ok(uploaded.id)
}

async fn patch(token: &str, id: &str, bytes: &[u8]) -> Result<(), DriveError> {
    let url = format!(
        "{UPLOAD_URL}/{}?uploadType=media&fields=id,version",
        encode(id)
    );
    http::send(
        "PATCH",
        &url,
        token,
        Some("application/json"),
        Some(bytes.to_vec()),
    )
    .await
    .map(|_| ())
}

async fn delete(token: &str, id: &str) -> Result<(), DriveError> {
    let url = format!("https://www.googleapis.com/drive/v3/files/{}", encode(id));
    http::send("DELETE", &url, token, None, None)
        .await
        .map(|_| ())
}

#[derive(Deserialize)]
struct Uploaded {
    id: String,
}

fn conflicts(file: &FileWrite, remote: &[RemoteFile]) -> bool {
    let current = remote.iter().find(|item| item.name == file.name);
    match (file.seen, current) {
        (None, None) => false,
        (Some(seen), Some(file)) => file.revision != seen,
        _ => true,
    }
}

fn multipart(name: &str, bytes: &[u8]) -> Vec<u8> {
    let metadata = serde_json::json!({
        "name": name,
        "parents": ["appDataFolder"],
    });
    let mut body = Vec::new();
    push(
        &mut body,
        "--buyometer\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n",
    );
    push(&mut body, &metadata.to_string());
    push(
        &mut body,
        "\r\n--buyometer\r\nContent-Type: application/json\r\n\r\n",
    );
    body.extend_from_slice(bytes);
    push(&mut body, "\r\n--buyometer--\r\n");
    body
}

fn push(body: &mut Vec<u8>, text: &str) {
    body.extend_from_slice(text.as_bytes());
}

fn encode(value: &str) -> String {
    js_sys::encode_uri_component(value).into()
}
