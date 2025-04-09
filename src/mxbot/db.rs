use super::state::{read_media_map, write_media_map, MediaCache, MediaHash, MediaMap};
use matrix_sdk::Client;
use mstickerlib::database::{self, Database};
use std::sync::Arc;
use tokio::sync::RwLock;

// TODO do we want to change this so that instead of a map in one account data, we instead
// store one data for each key?
#[must_use]
pub(super) struct AccountDataDatabase {
	map: Arc<RwLock<MediaMap>>
}

impl AccountDataDatabase {
	pub(super) async fn load(client: &Client) -> anyhow::Result<Self> {
		let map = read_media_map(client).await?.unwrap_or_default();
		Ok(Self {
			map: Arc::new(RwLock::new(map))
		})
	}

	pub(super) async fn store(self, client: &Client) -> anyhow::Result<()> {
		let map = self.map.read().await;
		write_media_map(client, &map).await
	}
}

//#[async_trait]
impl Database for AccountDataDatabase {
	async fn get(&self, hash: &database::Hash) -> anyhow::Result<Option<String>> {
		let map = self.map.read().await;
		Ok(map.map.get(hash).map(|cache| cache.url.clone()))
	}

	async fn add(&self, hash: database::Hash, url: String) -> anyhow::Result<()> {
		let mut map = self.map.write().await;
		map.map.insert(MediaHash(hash), MediaCache { url });
		Ok(())
	}
}
