use super::state::read_stickerpack;
use crate::mxbot::state::write_room_state;
use anyhow::{bail, Context as _};
use heck::ToSnakeCase;
use indexmap::IndexMap;
use log::info;
use matrix_sdk::room::Joined;
use mstickerlib::matrix::sticker_formats::{maunium, ponies};
use reqwest::header::{ACCEPT, USER_AGENT};

const MAX_CONTENT_LENGTH: usize = 100 * 1024;

pub(super) async fn migrate(room: &Joined, pack: &str) -> anyhow::Result<()> {
	let mut response = reqwest::Client::new()
		.get(pack)
		.header(ACCEPT, "application/json")
		.header(USER_AGENT, "tg2mx_bot")
		.send()
		.await
		.context("Failed to download maunium sticker pack")?;

	if response.content_length().unwrap_or(0) > MAX_CONTENT_LENGTH as u64 {
		bail!("Maximum content length exceeded");
	}

	let mut bytes = Vec::new();
	while let Some(chunk) = response
		.chunk()
		.await
		.context("Failed to download maunium sticker pack")?
	{
		if bytes.len() + chunk.len() > MAX_CONTENT_LENGTH {
			bail!("Maximum content length exceeded");
		}
		bytes.extend_from_slice(&chunk);
	}
	let maunium_pack: maunium::StickerPack =
		serde_json::from_slice(&bytes).context("Failed to parse maunium sticker pack")?;
	let mut id = maunium_pack.id.to_snake_case();
	id.retain(|ch| ch.is_alphanumeric());

	if read_stickerpack(room, &id)
		.await
		.context("Failed to check if sticker pack was already added to the room")?
		.is_some()
	{
		info!("Skipping import of {id} sticker pack");
		return Ok(());
	}

	let mut stickerpack = ponies::StickerPack {
		images: IndexMap::new(),
		pack: ponies::PackInfo {
			display_name: maunium_pack.title,
			avatar_url: None
		}
	};
	for sticker in maunium_pack.stickers {
		stickerpack.images.insert(sticker.id, ponies::Sticker {
			body: sticker.body,
			info: sticker.info.image_info,
			url: sticker.url,
			usage: [ponies::Usage::Sticker].into_iter().collect()
		});
	}

	write_room_state(room, "im.ponies.room_emotes", Some(&id), stickerpack)
		.await
		.context("Failed to add the sticker pack to the room")?;
	Ok(())
}
