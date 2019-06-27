use std::fs;
use std::io::{self, BufReader, BufWriter, Write};
use std::path::PathBuf;

use aes_ctr::stream_cipher::generic_array::typenum::uint::Unsigned;
use aes_ctr::stream_cipher::{NewStreamCipher, SyncStreamCipher};

use failure::{Error, Fail};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::api;

lazy_static! {
    static ref LOGIN_JSON_PATH: PathBuf = {
        let data_local_dir =
            dirs::data_local_dir().expect("unexpected: data_local_dir is not None");
        data_local_dir
            .join(env!("CARGO_PKG_NAME"))
            .join("login.json")
    };
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub access_token: api::AccessToken,
    pub password: Option<String>,
}

#[derive(Fail, Debug)]
#[fail(
    display = "{} is occurred when loading {:?}. please login first.",
    _0, _1
)]
pub struct ConfigLoadError(io::Error, PathBuf);

impl Config {
    pub fn save(&self) -> Result<(), io::Error> {
        if let Some(parent) = LOGIN_JSON_PATH.parent() {
            fs::create_dir_all(parent)?;
        };

        let file = fs::File::create(&*LOGIN_JSON_PATH)?;
        let mut bw = BufWriter::new(file);
        serde_json::to_writer(&mut bw, self)?;
        bw.flush()
    }

    pub fn load() -> Result<Config, Error> {
        let f = fs::File::open(&*LOGIN_JSON_PATH)
            .map_err(|e| ConfigLoadError(e, LOGIN_JSON_PATH.to_owned()))?;
        Ok(serde_json::from_reader(BufReader::new(f))?)
    }

    pub fn cipher_gen(&self) -> Result<Option<CipherGen>, Error> {
        match &self.password {
            None => Ok(None),
            Some(p) => {
                let mut pwd = vec![0; 256 / 8];
                faster_hex::hex_decode(p.as_bytes(), &mut pwd)?;
                Ok(Some(CipherGen::new(
                    pwd,
                    crate::app::NONCE.to_owned().to_vec(),
                )?))
            }
        }
    }
}

pub struct CipherGen {
    key: Vec<u8>,
    nonce: Vec<u8>,
}

#[derive(Fail, Debug)]
#[fail(display = "invalid key and/or nonce length")]
pub struct InvalidKeyNonceLength;

impl CipherGen {
    fn new(key: Vec<u8>, nonce: Vec<u8>) -> Result<Self, InvalidKeyNonceLength> {
        if key.len() != <aes_ctr::Aes256Ctr as NewStreamCipher>::KeySize::to_usize() {
            return Err(InvalidKeyNonceLength);
        }
        if nonce.len() != <aes_ctr::Aes256Ctr as NewStreamCipher>::NonceSize::to_usize() {
            return Err(InvalidKeyNonceLength);
        }
        Ok(CipherGen {
            key: key,
            nonce: nonce,
        })
    }

    pub fn cipher(&self) -> impl SyncStreamCipher {
        aes_ctr::Aes256Ctr::new_var(&self.key, &self.nonce).expect("valid size key and nonce")
    }
}

impl From<api::AuthorizeResponse> for Config {
    fn from(r: api::AuthorizeResponse) -> Config {
        Config {
            access_token: r.access_token,
            password: None,
        }
    }
}
