use failure::Error;
use read_input::prelude::*;
use reqwest::Client;

use crate::api;
use crate::config::Config;

fn auth(cli: Client, no_browser: bool) -> Result<api::AuthorizeResponse, reqwest::Error> {
    let url = api::authorize_url();
    if no_browser {
        log::info!("please open {}", url);
    } else if let Err(err) = webbrowser::open(&url) {
        log::error!("cannot open browser: {}\nplease open {}", err, url);
    };
    let token: String = input().msg("please type token: ").get();
    api::authorize(&cli, &token)
}

pub fn run(no_browser: bool) -> Result<(), Error> {
    let resp = auth(Client::new(), no_browser)?;
    let mut config = Config::load().unwrap_or_else(|_| Config::new());
    config.access_token = resp.access_token;
    config.save()?;
    log::info!("logged-in");
    Ok(())
}
