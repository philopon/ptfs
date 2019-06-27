use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use failure::Error;
use reqwest::{Client, ClientBuilder};

use crate::api;
use crate::config::Config;
use crate::crypto::CipherWrite;

pub struct ServerBuilder<D = ()> {
    _timeout: u64,
    _dst: D,
    _retry_wait: Duration,
}

impl ServerBuilder<()> {
    pub fn new() -> ServerBuilder<()> {
        ServerBuilder {
            _timeout: 30,
            _dst: (),
            _retry_wait: Duration::from_secs(10),
        }
    }
}

impl<D> ServerBuilder<D> {
    pub fn dst(self, dst: PathBuf) -> ServerBuilder<PathBuf> {
        ServerBuilder {
            _timeout: self._timeout,
            _dst: dst,
            _retry_wait: self._retry_wait,
        }
    }

    pub fn timeout(mut self, timeout: u64) -> ServerBuilder<D> {
        self._timeout = timeout;
        self
    }

    pub fn retry_wait(mut self, retry_wait: Duration) -> ServerBuilder<D> {
        self._retry_wait = retry_wait;
        self
    }
}

impl ServerBuilder<PathBuf> {
    pub fn build(self) -> Server {
        Server {
            timeout: self._timeout,
            dst: self._dst,
            retry_wait: self._retry_wait,
            cli: ClientBuilder::new()
                .timeout(Duration::from_secs(self._timeout + 60))
                .build()
                .expect("build Client from ClientBuilder"),
            access_token: api::AccessToken::new(),
        }
    }
}

#[derive(Clone)]
pub struct Server {
    timeout: u64,
    dst: PathBuf,
    retry_wait: Duration,
    cli: Client,
    access_token: api::AccessToken,
}

impl Server {
    fn backoff(&self, err: Error, wait: Duration) {
        log::info!("{}\nwait {:?}", err, wait);
        thread::sleep(wait);
        log::info!("retry");
    }

    fn longpoll(&self, cursor: &api::Cursor) {
        loop {
            match api::list_folder_longpoll(&self.cli, self.timeout, cursor) {
                Err(err) => self.backoff(err, self.retry_wait),
                Ok(lp) => {
                    if lp.changes {
                        return;
                    }
                    if let Some(backoff) = lp.backoff {
                        thread::sleep(Duration::from_secs(backoff))
                    }
                }
            }
        }
    }

    fn list(&self) -> api::ListFolderResponse {
        loop {
            match api::list_folder(&self.cli, &self.access_token) {
                Ok(r) => return r,
                Err(err) => self.backoff(err.into(), self.retry_wait),
            }
        }
    }

    fn issue_file(&self, name: &str) -> (fs::File, PathBuf) {
        let mut i = 0;
        loop {
            let path = self.dst.join(if i == 0 {
                name.to_owned()
            } else {
                match name.rfind('.') {
                    None => format!("{} ({})", name, i),
                    Some(e) => format!("{} ({}){}", &name[..e], i, &name[e..]),
                }
            });

            if !path.exists() {
                match fs::File::create(&path) {
                    Ok(f) => return (f, path),
                    Err(_) => log::warn!("cannot create {:?}", path),
                }
            }

            i += 1;
        }
    }

    pub fn run(mut self) -> Result<(), Error> {
        let config = Config::load()?;
        let gen = config.cipher_gen()?;
        self.access_token = config.access_token;

        let (send, recv) = mpsc::channel();
        let this = self.clone();
        thread::spawn(move || {
            let mut list_folder = this.list();
            loop {
                list_folder.entries.sort_unstable();
                for entry in list_folder.entries.into_iter() {
                    match entry {
                        api::Entry::File(file) => {
                            match send.send(file) {
                                Ok(_) => {}
                                Err(e) => log::error!("cannot send to channel: {}", e),
                            };
                        }
                        _ => {}
                    }
                }
                this.longpoll(&list_folder.cursor);
                list_folder = this.list();
            }
        });

        loop {
            let entry = recv.recv().expect("unexpected: chanel is closed");
            let (dst, dst_path) = self.issue_file(&entry.name);
            let mut bw = BufWriter::new(dst);
            let result = match gen {
                Some(ref gen) => api::download(
                    &self.cli,
                    &self.access_token,
                    &entry.id,
                    &mut CipherWrite::new(gen.cipher(), bw),
                ),
                None => api::download(&self.cli, &self.access_token, &entry.id, &mut bw),
            };

            match result {
                Ok(hash) => {
                    if Some(hash) != entry.content_hash {
                        log::error!(
                            "content_hash mismatched in {}. expected: {:?}, actual: {:?}",
                            &entry.path_display,
                            entry.content_hash,
                            entry.id
                        );
                        fs::remove_file(dst_path)?;
                        continue;
                    }
                    log::info!(
                        "{} was downloaded to {}",
                        &entry.path_display,
                        dst_path.display()
                    );
                    match api::delete(&self.cli, &self.access_token, &entry.id) {
                        Ok(_) => log::info!("deleted {} from Dropbox", &entry.path_display),
                        Err(e) => self.backoff(e.into(), self.retry_wait),
                    }
                }
                Err(e) => self.backoff(e.into(), self.retry_wait),
            }
        }
    }
}
