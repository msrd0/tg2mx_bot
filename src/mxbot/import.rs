use crate::{
	mxbot::{db::AccountDataDatabase, state::write_room_state},
	TG_BOT_TOKEN
};
use anyhow::{anyhow, Context as _};
use heck::ToSnakeCase;
use log::{error, warn};
use matrix_sdk::room::Room;
use mstickerlib::{
	image::AnimationFormat,
	matrix::{self, sticker_formats::ponies},
	tg::{self, ImportConfig}
};

pub(super) async fn import(room: &Room, pack: &str) -> anyhow::Result<()> {
	let pack = tg::pack_url_to_name(pack).context("Invalid sticker pack url")?;
	let mut id = pack.to_snake_case();
	id.retain(|ch| ch.is_alphanumeric());

	// config to connect to telegram
	let tg_config = tg::Config {
		bot_key: TG_BOT_TOKEN.as_deref().unwrap().to_owned()
	};

	// config to connect to matrix
	let client = room.client();
	let matrix_config = matrix::Config {
		homeserver_url: client.homeserver().to_string(),
		user: client
			.user_id()
			.ok_or_else(|| anyhow!("Unable to obtain my own matrix user id"))?
			.to_string(),
		access_token: client
			.access_token()
			.ok_or_else(|| anyhow!("Unable to obtain my own matrix access token"))?
	};

	// database, stored in the matrix account data, to prevent duplicate file uploads
	let db = AccountDataDatabase::load(&client)
		.await
		.context("Failed to load database from account data")?;

	// load the telegram sticker pack
	let sticker_pack = tg::StickerPack::get(pack, &tg_config)
		.await
		.context("Failed to load the sticker pack from telegram")?;

	// import the pack to matrix
	let mut import_config = ImportConfig::default();
	import_config.animation_format = AnimationFormat::Webp;
	import_config.database = Some(&db);
	let matrix_pack = match sticker_pack
		.import(&tg_config, &matrix_config, &import_config)
		.await
	{
		Ok(matrix_pack) => matrix_pack,
		Err((matrix_pack, errors)) => {
			// print warnings for those stickers from the set that were ignored
			// TODO these warnings should be printed to matrix
			for (i, err) in errors {
				warn!("Failed to import sticker {i}: {err:?}");
			}

			matrix_pack
		}
	};

	let ponies: ponies::StickerPack = matrix_pack.into();
	write_room_state(room, "im.ponies.room_emotes", Some(&id), ponies)
		.await
		.context("Failed to add the sticker pack to the room")?;

	// store the changes to the database
	// TODO do we need some kind of synchronisation/locking here?
	if let Err(err) = db.store(&client).await {
		error!("Unable to store database to account data: {err:?}");
	}

	Ok(())
}
