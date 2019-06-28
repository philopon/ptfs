use std::io::{self, BufReader, BufWriter, Read, Write};

use aes_ctr::stream_cipher::SyncStreamCipher;
use failure::Error;
use reqwest::Client;
use sha2::{Digest, Sha256};
use rustc_hex::ToHex;

use crate::api;
use crate::config::Config;

pub enum Mode {
    Enable,
    Disable,
    Apply,
}

pub fn run(enable: Mode) -> Result<(), Error> {
    let config = Config::load()?;
    match enable {
        Mode::Enable => enable_crypto(config),
        Mode::Disable => disable_crypto(config),
        Mode::Apply => apply(config),
    }
}

fn disable_crypto(mut config: Config) -> Result<(), Error> {
    config.password = None;
    config.save()?;
    log::info!("crypto file disabled");
    Ok(())
}

fn enable_crypto(mut config: Config) -> Result<(), Error> {
    let cli = Client::new();
    let account = api::get_current_account(&cli, &config.access_token)?;
    let mut hasher = Sha256::new();
    hasher.input(&account.account_id);
    let password = rpassword::read_password_from_tty(Some("type encrypto password: "))?;
    hasher.input(&password);
    let hashed = hasher.result().to_vec().to_hex();
    config.password = Some(hashed);
    config.save()?;
    log::info!("crypto file enabled");
    Ok(())
}

fn apply(config: Config) -> Result<(), Error> {
    if let Some(gen) = config.cipher_gen()? {
        let mut r = CipherRead::new(gen.cipher(), BufReader::new(io::stdin()));
        let mut w = BufWriter::new(io::stdout());
        io::copy(&mut r, &mut w)?;
    };

    Ok(())
}

pub struct CipherRead<C, R> {
    cipher: C,
    inner: R,
}

impl<C, R> CipherRead<C, R> {
    pub fn new(cipher: C, inner: R) -> Self {
        CipherRead {
            cipher: cipher,
            inner: inner,
        }
    }
}

impl<C: SyncStreamCipher, R: Read> Read for CipherRead<C, R> {
    fn read(&mut self, mut bytes: &mut [u8]) -> io::Result<usize> {
        let len = self.inner.read(bytes)?;
        self.cipher.apply_keystream(&mut bytes);
        Ok(len)
    }
}

pub struct CipherWrite<C, W> {
    cipher: C,
    inner: W,
}

impl<C, W> CipherWrite<C, W> {
    pub fn new(cipher: C, inner: W) -> Self {
        CipherWrite {
            cipher: cipher,
            inner: inner,
        }
    }
}

impl<C: SyncStreamCipher, W: Write> Write for CipherWrite<C, W> {
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn write(&mut self, src: &[u8]) -> io::Result<usize> {
        let mut v = src.to_vec();
        self.cipher.apply_keystream(&mut v);
        self.inner.write(&v)
    }
}
