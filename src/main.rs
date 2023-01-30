use ruma::{client::http_client::Reqwest, Client};
use std::env;

fn env(name: &str) -> String {
	env::var(name).unwrap_or_else(|_| panic!("Missing {name} environment variable"))
}

async fn run() -> anyhow::Result<()> {
	let homeserver = env("HOMESERVER");
	let matrix_id = env("MATRIX_ID");
	let access_token = env("ACCESS_TOKEN");

	let client = Client::builder()
		.homeserver_url(homeserver)
		.access_token(Some(access_token))
		.build::<Reqwest>()
		.await?;

	Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	run().await
}
