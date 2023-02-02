use anyhow::bail;
use matrix_sdk::Client;
use mstickerlib::matrix::sticker_formats::maunium::StickerPack;
use reqwest::header::{ACCEPT, USER_AGENT};

const MAX_CONTENT_LENGTH: usize = 50 * 1024;

pub(super) async fn migrate(client: &Client, pack: &str) -> anyhow::Result<()> {
	let mut response = reqwest::Client::new()
		.get(pack)
		.header(ACCEPT, "application/json")
		.header(USER_AGENT, "tg2mx_bot")
		.send()
		.await?;

	if response.content_length().unwrap_or(0) > MAX_CONTENT_LENGTH as u64 {
		bail!("Maximum content length exceeded");
	}

	let mut bytes = Vec::new();
	while let Some(chunk) = response.chunk().await? {
		if bytes.len() + chunk.len() > MAX_CONTENT_LENGTH {
			bail!("Maximum content length exceeded");
		}
		bytes.extend_from_slice(&chunk);
	}
	let maunium_pack: StickerPack = serde_json::from_slice(&bytes)?;

	bail!("unimplemented")
}
