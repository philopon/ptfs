pub const AUTHORIZE: &'static str = "https://www.dropbox.com/oauth2/authorize";
pub const OAUTH2_TOKEN: &'static str = "https://api.dropboxapi.com/oauth2/token";

pub const LIST_FOLDER: &'static str = "https://api.dropboxapi.com/2/files/list_folder";
pub const LIST_FOLDER_LONGPOLL: &'static str =
    "https://notify.dropboxapi.com/2/files/list_folder/longpoll";

pub const DOWNLOAD: &'static str = "https://content.dropboxapi.com/2/files/download";

pub const DELETE: &'static str = "https://api.dropboxapi.com/2/files/delete_v2";

pub const UPLOAD_SESSION_START: &'static str =
    "https://content.dropboxapi.com/2/files/upload_session/start";
pub const UPLOAD_SESSION_FINISH: &'static str =
    "https://content.dropboxapi.com/2/files/upload_session/finish";
pub const UPLOAD_SESSION_APPEND: &'static str =
    "https://content.dropboxapi.com/2/files/upload_session/append_v2";

pub const GET_CURRENT_ACCOUNT: &'static str =
    "https://api.dropboxapi.com/2/users/get_current_account";

pub const CONTENT_HASH_BLOCK_SIZE: usize = 4 * 1024 * 1024;
