use log::{error, info};
use matrix_sdk::{
	room::Joined,
	ruma::{
		events::{
			room::message::OriginalRoomMessageEvent, MessageLikeEventContent,
			OriginalMessageLikeEvent
		},
		serde::Raw,
		MilliSecondsSinceUnixEpoch, OwnedEventId, OwnedRoomId, OwnedUserId
	},
	Client
};
use monostate::MustBe;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::VecDeque;

pub(super) async fn read_account_data<T>(
	client: &Client,
	key: &str
) -> anyhow::Result<serde_json::Result<Option<T>>>
where
	T: DeserializeOwned
{
	Ok(client
		.account()
		.account_data_raw(key.into())
		.await?
		.map(|ev| ev.deserialize_as())
		.transpose())
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
	pub(super) q: VecDeque<QueuedJob>
}

/// because serde always passes an argument
fn default<T, U: Default>(_this: &T) -> U {
	U::default()
}

#[rustfmt::skip] // sorry but rustfmt can't handle comments in structs
#[derive(Serialize)]
#[serde(remote = "OriginalMessageLikeEvent")]
struct OriginalMessageLikeEventDef<C: MessageLikeEventContent> {
	content: C,
	event_id: OwnedEventId,
	sender: OwnedUserId,
	origin_server_ts: MilliSecondsSinceUnixEpoch,
	room_id: OwnedRoomId,

	// unsigned ignored as we don't care if we use the actual data or the default value

	// we need to serialize the message type eventhough it's not included
	#[serde(rename = "type", getter = "default")]
	ty: MustBe!("m.room.message")
}

#[derive(Deserialize, Serialize)]
pub(super) struct QueuedJob {
	#[serde(serialize_with = "OriginalMessageLikeEventDef::serialize")]
	pub(super) ev: OriginalRoomMessageEvent,

	pub(super) job: Job
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "pack")]
pub(super) enum Job {
	Import(String),
	Migrate(String)
}

pub(super) async fn read_queue(client: &Client) -> anyhow::Result<Queue> {
	Ok(read_account_data(client, "de.msrd0.tg2mx_bot.queue")
		.await?
		.unwrap_or_else(|err| {
			error!("Failed to deserialize account data: {err}");
			None
		})
		.map(|q: Queue| {
			info!("Read queue with {} jobs", q.q.len());
			q
		})
		.unwrap_or_default())
}

pub(super) async fn write_queue(client: &Client, q: &Queue) -> anyhow::Result<()> {
	info!("Writing queue with {} jobs", q.q.len());
	write_account_data(client, "de.msrd0.tg2mx_bot.queue", q).await?;
	if read_queue(client).await?.q.len() != q.q.len() {
		panic!("WTF?!?");
	}
	Ok(())
}
