use super::state::{read_media_map, write_media_map, MediaCache, MediaHash, MediaMap};
use matrix_sdk::Client;
use mstickerlib::database::{self, Database};
use parking_lot::RwLock;
use std::sync::Arc;

#[must_use]
pub struct AccountDataDatabase {
	map: Arc<RwLock<MediaMap>>
}

impl AccountDataDatabase {
	async fn load(client: &Client) -> anyhow::Result<Self> {
		let map = read_media_map(client).await?.unwrap_or_default();
		Ok(Self {
			map: Arc::new(RwLock::new(map))
		})
	}

	async fn store(self, client: &Client) -> anyhow::Result<()> {
		// TODO fix non-async RwLock
		let map = self.map.read();
		write_media_map(client, &map).await
	}
}

impl Database for AccountDataDatabase {
	fn get(&self, hash: &database::Hash) -> Option<String> {
		let map = self.map.read();
		map.map.get(hash).map(|cache| cache.url.clone())
	}

	fn add(&self, hash: database::Hash, url: String) -> anyhow::Result<()> {
		let mut map = self.map.write();
		map.map.insert(MediaHash(hash), MediaCache { url });
		Ok(())
	}
}
