use super::state::{read_media_map, write_media_map, MediaMap};
use matrix_sdk::Client;
use mstickerlib::database::{self, Database};

#[must_use]
pub struct AccountDataDatabase {
	map: MediaMap
}

impl AccountDataDatabase {
	async fn load(client: &Client) -> anyhow::Result<Self> {
		let map = read_media_map(client).await?.unwrap_or_default();
		Ok(Self { map })
	}

	async fn store(self, client: &Client) -> anyhow::Result<()> {
		write_media_map(client, &self.map).await
	}
}

impl Database for AccountDataDatabase {
	fn get(&self, hash: &database::Hash) -> Option<String> {
		self.map.map.get(hash).map(|cache| cache.url.clone())
	}

	fn add(&self, hash: database::Hash, url: String) -> anyhow::Result<()> {
		unimplemented!()
	}
}
