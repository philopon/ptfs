use std::fs::{self, File};
use std::io::{self, Read, BufReader};
use std::path::PathBuf;
use std::time::Duration;

use failure::{Error, Fail};
use indicatif::ProgressBar;
use reqwest::{Client, ClientBuilder};

use crate::api;
use crate::config::{Config, CipherGen};
use crate::crypto::CipherRead;

const CHUNK: usize = 4 * 1024 * 1024;

#[derive(Fail, Debug)]
#[fail(display = "cannot get file name of {:?}", _0)]
struct CreateNameError(PathBuf);


fn download_read<B: Read>(
    cli: &Client,
    access_token: &api::AccessToken,
    gen: &Option<CipherGen>,
    name: &str,
    body: B,
) -> Result<(), Error> {
    match gen {
        Some(gen) => api::upload(
            &cli,
            access_token,
            CipherRead::new(gen.cipher(), body),
            name,
            CHUNK,
        )?,
        None => api::upload(&cli, access_token, body, name, CHUNK)?,
    };
    Ok(())
}

fn download_file(
    cli: &Client,
    access_token: &api::AccessToken,
    gen: &Option<CipherGen>,
    path: &PathBuf,
    quiet: bool,
) -> Result<(), Error> {
    let pb = if quiet {
        ProgressBar::hidden()
    } else {
        let meta = fs::metadata(path)?;
        ProgressBar::new(meta.len())
    };

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| CreateNameError(path.to_owned()))?;
    let name = &format!("/{}", name);

    download_read(
        cli,
        access_token,
        gen,
        &name,
        BufReader::new(pb.wrap_read(File::open(path)?)),
    )
}


pub fn run(paths: &[PathBuf], name: &str, quiet: bool) -> Result<(), Error> {
    let cli = ClientBuilder::new()
        .timeout(Duration::from_secs(10 * 60))
        .build()
        .expect("build Client from ClientBuilder");

    let config = Config::load()?;
    let cipher_gen = config.cipher_gen()?;

    for path in paths {
        match download_file(&cli, &config.access_token, &cipher_gen, path, quiet) {
            Ok(()) => log::info!("{} is uploaded to Dropbox", path.display()),
            Err(e) => log::error!("{} is not uploaded to Dropbox: {}", path.display(), e),
        }
    }

    if atty::isnt(atty::Stream::Stdin) {
        match download_read(
            &cli,
            &config.access_token,
            &cipher_gen,
            &format!("/{}", name),
            io::stdin(),
        ) {
            Ok(()) => log::info!("{} is uploaded to Dropbox", name),
            Err(e) => log::error!("{} is not uploaded to Dropbox: {}", name, e),
        }
    }

    Ok(())
}
