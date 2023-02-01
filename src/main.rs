#![warn(rust_2018_idioms, unreachable_pub)]
#![deny(elided_lifetimes_in_paths, unsafe_code)]

use anyhow::anyhow;
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use std::env;

mod mxbot;

macro_rules! env {
	($($var:ident),*) => {
		$(
			static $var: Lazy<anyhow::Result<String>> = Lazy::new(|| {
				env::var(stringify!($var)).map_err(|_| {
					anyhow!("Missing {} environment variable", stringify!($var))
				})
			});
		)*
	};
}

env! {
	ADMIN,
	HOMESERVER,
	MATRIX_ID,
	PASSWORD
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	pretty_env_logger::init_timed();
	dotenv().ok();

	mxbot::run().await
}
