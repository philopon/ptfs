use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use failure::{Error, Fail};
use indicatif::ProgressBar;
use reqwest::ClientBuilder;

use crate::api;
use crate::config::Config;
use crate::crypto::CipherRead;

#[derive(Fail, Debug)]
#[fail(display = "cannot get file name of {:?}", _0)]
struct CreateNameError(PathBuf);

pub fn run(paths: &[PathBuf], quiet: bool) -> Result<(), Error> {
    let cli = ClientBuilder::new()
        .timeout(Duration::from_secs(10 * 60))
        .build()
        .expect("build Client from ClientBuilder");

    let config = Config::load()?;
    let cipher_gen = Rc::new(config.cipher_gen()?);

    for path in paths {
        let gen = cipher_gen.clone();
        let result = (|| -> Result<(), Error> {
            let pb = if quiet {
                ProgressBar::hidden()
            } else {
                let meta = fs::metadata(path)?;
                ProgressBar::new(meta.len())
            };

            let file = File::open(path)?;
            let br = BufReader::new(file);
            let br = pb.wrap_read(br);
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| CreateNameError(path.to_owned()))?;
            let name = &format!("/{}", name);
            let chunk = 4 * 1024 * 1024;

            match gen.as_ref() {
                Some(gen) => api::upload(
                    &cli,
                    &config.access_token,
                    CipherRead::new(gen.cipher(), br),
                    name,
                    chunk,
                )?,
                None => api::upload(&cli, &config.access_token, br, name, chunk)?,
            };
            Ok(())
        })();

        match result {
            Ok(()) => log::info!("{} is uploaded to Dropbox", path.display()),
            Err(e) => log::error!("{} is not uploaded to Dropbox: {}", path.display(), e),
        }
    }

    Ok(())
}
