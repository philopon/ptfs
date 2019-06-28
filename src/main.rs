use std::path::PathBuf;
use std::process;
use std::time::Duration;

use structopt::StructOpt;
use lazy_static::lazy_static;

mod api;
mod app;
mod config;
mod crypto;
mod download;
mod login;
mod server;
mod url;

lazy_static! {
    static ref DEFAULT_DST: String = {
        match dirs::download_dir() {
            Some(dl) => format!("{}", dl.display()),
            None => ".".to_string(),
        }
    };
}

#[derive(Debug, StructOpt)]
enum Opt {
    #[structopt(name = "login", about = "login to dropbox")]
    Login {
        #[structopt(long = "--no-browser", help = "don't open web browser")]
        no_browser: bool,
    },
    #[structopt(name = "server", about = "start dl-watcher server")]
    Server {
        #[structopt(
            name = "DST",
            help = "download directory",
            raw(default_value = "&DEFAULT_DST")
        )]
        dst: PathBuf,
        #[structopt(
            short = "-t",
            long = "--timeout",
            help = "timeout",
            default_value = "300"
        )]
        timeout: u64,
        #[structopt(
            short = "-r",
            long = "--retry-wait",
            help = "retry wait",
            default_value = "10"
        )]
        retry_wait: u64,
    },
    #[structopt(name = "crypto", about = "enable/disable crypto file")]
    Crypto(CryptoOpt),
    #[structopt(name = "download", about = "download file(s)")]
    Download {
        #[structopt(name = "FILE", help = "download file(s)")]
        paths: Vec<PathBuf>,
        #[structopt(short = "-q", long = "--quiet", help = "don't display progress bar")]
        quiet: bool,
        #[structopt(
            short = "-n",
            long = "--name",
            help = "stdin file name",
            default_value = "stdin"
        )]
        name: String,
    },
}

#[derive(Debug, StructOpt)]
enum CryptoOpt {
    #[structopt(name = "enable", about = "enable crypto file")]
    Enable,
    #[structopt(name = "disable", about = "disable crypto file")]
    Disable,
    #[structopt(name = "apply", about = "encrypto/decrypto stdin to stdout")]
    Apply,
}

impl Into<crypto::Mode> for CryptoOpt {
    fn into(self) -> crypto::Mode {
        match self {
            CryptoOpt::Enable => crypto::Mode::Enable,
            CryptoOpt::Disable => crypto::Mode::Disable,
            CryptoOpt::Apply => crypto::Mode::Apply,
        }
    }
}

fn main() {
    let opt = Opt::from_args();
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let res = match opt {
        Opt::Login { no_browser } => login::run(no_browser),
        Opt::Server {
            dst,
            timeout,
            retry_wait,
        } => server::ServerBuilder::new()
            .dst(dst)
            .timeout(timeout)
            .retry_wait(Duration::from_secs(retry_wait))
            .build()
            .run(),
        Opt::Download { paths, quiet, name } => download::run(&paths, &name, quiet),
        Opt::Crypto(flag) => crypto::run(flag.into()),
    };

    let exitcode = match res {
        Ok(()) => 0,
        Err(err) => {
            for e in err.iter_chain() {
                log::error!("{}", e);
            }
            1
        }
    };

    process::exit(exitcode);
}
