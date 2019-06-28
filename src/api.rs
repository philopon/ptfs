use std::io::{Read, Write};

use failure::{Error, Fail};
use lazy_static::lazy_static;
use reqwest::{header, header::HeaderName, Body, Client};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use rustc_hex::ToHex;

use crate::app;
use crate::url;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AccessToken(String);

lazy_static! {
    static ref DROPBOX_API_ARG: HeaderName = header::HeaderName::from_lowercase(b"dropbox-api-arg")
        .expect("create Dropbox-API-Arg header");
}

impl AccessToken {
    pub fn new() -> AccessToken {
        AccessToken("".to_string())
    }
}

impl std::fmt::Display for AccessToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        self.0.fmt(formatter)
    }
}

pub fn authorize_url() -> String {
    return format!(
        "{}?client_id={}&response_type=code",
        url::AUTHORIZE,
        app::KEY
    );
}

#[derive(Debug, Deserialize)]
pub struct AuthorizeResponse {
    pub access_token: AccessToken,
}

pub fn authorize(cli: &Client, token: &str) -> Result<AuthorizeResponse, reqwest::Error> {
    cli.post(url::OAUTH2_TOKEN)
        .form(&[("code", token), ("grant_type", "authorization_code")])
        .basic_auth(app::KEY, Some(app::SECRET))
        .send()?
        .error_for_status()?
        .json()
        .map_err(Into::into)
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path(pub String);

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentHash(String);

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileEntry {
    pub name: String,
    pub id: Path,
    pub path_display: String,
    pub size: usize,
    pub content_hash: Option<ContentHash>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = ".tag")]
pub enum Entry {
    #[serde(rename = "file")]
    File(FileEntry),
    #[serde(rename = "folder")]
    Folder,
}

#[derive(Debug, Deserialize)]
pub struct Cursor(String);

#[derive(Debug, Deserialize)]
pub struct ListFolderResponse {
    pub entries: Vec<Entry>,
    pub cursor: Cursor,
    pub has_more: bool,
}

pub fn list_folder(
    cli: &Client,
    access_token: &AccessToken,
) -> Result<ListFolderResponse, reqwest::Error> {
    cli.post(url::LIST_FOLDER)
        .bearer_auth(access_token)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&json!({"path": ""}))
        .send()?
        .error_for_status()?
        .json()
        .map_err(Into::into)
}

#[derive(Debug, Deserialize)]
pub struct LongPollResponse {
    pub changes: bool,
    pub backoff: Option<u64>,
}

#[derive(Fail, Debug)]
#[fail(display = "{}", _0)]
struct APIParameterError(String);

pub fn list_folder_longpoll(
    cli: &Client,
    timeout: u64,
    cursor: &Cursor,
) -> Result<LongPollResponse, Error> {
    if timeout < 30 {
        Err(APIParameterError(
            "timeout should larger than or equal to 30".to_string(),
        ))?;
    };
    if timeout > 480 {
        Err(APIParameterError(
            "timeout should less than or equal to 480".to_string(),
        ))?;
    };
    cli.post(url::LIST_FOLDER_LONGPOLL)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&json!({"cursor": cursor.0, "timeout": timeout}))
        .send()?
        .error_for_status()?
        .json()
        .map_err(Into::into)
}

pub fn delete(cli: &Client, access_token: &AccessToken, path: &Path) -> Result<(), reqwest::Error> {
    cli.post(url::DELETE)
        .bearer_auth(access_token)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&json!({"path": path.0}))
        .send()?
        .error_for_status()?;
    Ok(())
}

pub fn download<W: Write>(
    cli: &Client,
    access_token: &AccessToken,
    path: &Path,
    dst: &mut W,
) -> Result<ContentHash, Error> {
    let mut resp = cli
        .post(url::DOWNLOAD)
        .bearer_auth(access_token)
        .header(&*DROPBOX_API_ARG, json!({ "path": path }).to_string())
        .send()?
        .error_for_status()?;
    let mut buf = Vec::with_capacity(url::CONTENT_HASH_BLOCK_SIZE);
    let mut hashes = Vec::new();
    loop {
        let len = resp
            .by_ref()
            .take(url::CONTENT_HASH_BLOCK_SIZE as u64)
            .read_to_end(&mut buf)?;
        if len <= 0 {
            break;
        }
        let mut hasher = Sha256::new();
        hasher.input(&buf);
        hashes.extend(hasher.result().to_vec());
        dst.write(&buf)?;

        buf.clear();
    }

    let mut hasher = Sha256::new();
    hasher.input(&hashes);
    Ok(ContentHash(hasher.result().to_vec().to_hex()))
}

#[derive(Debug, Deserialize, Serialize)]
struct SessionId(String);

#[derive(Debug, Deserialize)]
struct UploadSessionStartResponse {
    session_id: SessionId,
}

fn upload_session_start<T: Into<Body>>(
    cli: &Client,
    access_token: &AccessToken,
    body: T,
    close: bool,
) -> Result<UploadSessionStartResponse, reqwest::Error> {
    cli.post(url::UPLOAD_SESSION_START)
        .bearer_auth(access_token)
        .header(&*DROPBOX_API_ARG, json!({ "close": close }).to_string())
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(body)
        .send()?
        .error_for_status()?
        .json()
        .map_err(Into::into)
}

#[derive(Debug, Serialize)]
struct UploadSessionCursor {
    session_id: SessionId,
    offset: usize,
}

#[derive(Debug, Serialize)]
struct UploadSessionConfig<'a> {
    cursor: &'a UploadSessionCursor,
    close: bool,
}

fn upload_session_append<T: Into<Body>>(
    cli: &Client,
    access_token: &AccessToken,
    body: T,
    config: UploadSessionConfig,
) -> Result<(), reqwest::Error> {
    cli.post(url::UPLOAD_SESSION_APPEND)
        .bearer_auth(access_token)
        .header(
            &*DROPBOX_API_ARG,
            serde_json::to_string(&config)
                .expect("valid Dropbox-API-Arg header of upload_session/append"),
        )
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(body)
        .send()?
        .error_for_status()?;
    Ok(())
}

#[derive(Debug, Serialize)]
enum UploadSessionFinishMode {
    #[serde(rename = "add")]
    Add,
}

#[derive(Debug, Serialize)]
struct UploadSessionFinishCommit<'a> {
    path: &'a str,
    mode: UploadSessionFinishMode,
    autorename: bool,
    mute: bool,
    strict_conflict: bool,
}

#[derive(Debug, Serialize)]
struct UploadSessionFinishConfig<'a> {
    cursor: &'a UploadSessionCursor,
    commit: UploadSessionFinishCommit<'a>,
}

fn upload_session_finish<T: Into<Body>>(
    cli: &Client,
    access_token: &AccessToken,
    body: T,
    config: UploadSessionFinishConfig,
) -> Result<(), reqwest::Error> {
    cli.post(url::UPLOAD_SESSION_FINISH)
        .bearer_auth(access_token)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            &*DROPBOX_API_ARG,
            serde_json::to_string(&config)
                .expect("valid Dropbox-API-Arg header of upload_session/append"),
        )
        .body(body)
        .send()?
        .error_for_status()?;
    Ok(())
}

pub fn upload<R: Read>(
    cli: &Client,
    access_token: &AccessToken,
    mut body: R,
    path: &str,
    chunk_size: usize,
) -> Result<(), Error> {
    let mut buf = vec![0; chunk_size];
    let len = body.read(&mut buf)?;
    buf.resize(len, 0);
    let session_id = upload_session_start(cli, access_token, buf, false)?.session_id;

    let mut cursor = UploadSessionCursor {
        session_id: session_id,
        offset: len,
    };

    loop {
        let mut buf = vec![0; chunk_size];
        let len = body.read(&mut buf)?;
        if len == 0 {
            break;
        }
        buf.resize(len, 0);
        upload_session_append(
            cli,
            access_token,
            buf,
            UploadSessionConfig {
                cursor: &cursor,
                close: false,
            },
        )?;
        cursor.offset += len;
    }

    upload_session_finish(
        cli,
        access_token,
        vec![],
        UploadSessionFinishConfig {
            cursor: &cursor,
            commit: UploadSessionFinishCommit {
                path: path,
                mode: UploadSessionFinishMode::Add,
                autorename: true,
                mute: true,
                strict_conflict: true,
            },
        },
    )?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct Account {
    pub account_id: String,
}

pub fn get_current_account(
    cli: &Client,
    access_token: &AccessToken,
) -> Result<Account, reqwest::Error> {
    cli.post(url::GET_CURRENT_ACCOUNT)
        .bearer_auth(access_token)
        .send()?
        .error_for_status()?
        .json()
        .map_err(Into::into)
}
