use failure::Error;
use read_input::prelude::*;
use reqwest::Client;

use crate::api;
use crate::config::Config;

fn auth(cli: Client) -> Result<api::AuthorizeResponse, reqwest::Error> {
    let url = api::authorize_url();
    if let Err(err) = webbrowser::open(&url) {
        log::error!("cannot open browser: {}\nplease open {}", err, url);
    };
    let token: String = input().msg("please type token: ").get();
    api::authorize(&cli, &token)
}

pub fn run() -> Result<(), Error> {
    let resp = auth(Client::new())?;
    Config::from(resp).save()?;
    log::info!("logged-in");
    Ok(())
}
