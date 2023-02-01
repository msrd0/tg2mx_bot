use matrix_sdk::{
	room::Joined,
	ruma::{events::StateEventContent, serde::Raw},
	Client
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::VecDeque;

pub(super) async fn read_account_data<T>(
	client: &Client,
	key: &str
) -> anyhow::Result<Option<T>>
where
	T: DeserializeOwned
{
	Ok(client
		.account()
		.account_data_raw(key.into())
		.await?
		.map(|ev| ev.deserialize_as())
		.transpose()?)
}

pub(super) async fn write_account_data<T>(
	client: &Client,
	key: &str,
	value: &T
) -> anyhow::Result<()>
where
	T: Serialize
{
	client
		.account()
		.set_account_data_raw(key.into(), Raw::new(value)?.cast())
		.await?;
	Ok(())
}

pub(super) async fn write_room_state<T>(
	room: Joined,
	key: &str,
	state_key: Option<&str>,
	content: T
) -> anyhow::Result<()>
where
	T: Serialize
{
	room.send_state_event_raw(
		serde_json::to_value(&content)?,
		key,
		state_key.unwrap_or("")
	)
	.await?;
	Ok(())
}

#[derive(Default, Deserialize, Serialize)]
pub(super) struct Queue {
	pub(super) q: VecDeque<Job>
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "job")]
pub(super) enum Job {
	Import(String),
	Migrate(String)
}

pub(super) async fn read_queue(client: &Client) -> anyhow::Result<Queue> {
	read_account_data(client, "de.msrd0.tg2mx_bot.queue")
		.await
		.map(|q| q.unwrap_or_default())
}

pub(super) async fn write_queue(client: &Client, q: &Queue) -> anyhow::Result<()> {
	write_account_data(client, "de.msrd0.tg2mx_bot.queue", q).await
}
