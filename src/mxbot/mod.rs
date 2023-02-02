use crate::{mxbot::state::write_queue, ADMIN, HOMESERVER, MATRIX_ID, PASSWORD};
use futures_util::future::select;
use indoc::indoc;
use log::{error, info, warn};
use matrix_sdk::{
	config::SyncSettings,
	room::{Joined, Room},
	ruma::events::{
		room::{
			member::StrippedRoomMemberEvent,
			message::{
				ForwardThread, MessageType, OriginalSyncRoomMessageEvent,
				RoomMessageEventContent
			}
		},
		MessageLikeEventContent
	},
	Client
};
use std::time::Duration;
use tokio::time::sleep;

mod import;
mod migrate;
mod state;

use import::import;
use migrate::migrate;
use ruma::events::{reaction::ReactionEventContent, relation::Annotation};
use state::{read_queue, Job};

async fn autojoin_handler(ev: StrippedRoomMemberEvent, room: Room, client: Client) {
	// ignore member events for other users
	if ev.state_key != client.user_id().unwrap() {
		return;
	}

	if let Room::Invited(room) = room {
		let room_id = room.room_id();

		// ignore events that weren't sent by an admin
		let sender = ev.sender.as_str();
		if !ADMIN
			.as_deref()
			.map(|admins| admins.split([',', ' ']).any(|admin| admin == sender))
			.unwrap_or(true)
		{
			warn!("Rejecting invitation for {room_id}");
			room.reject_invitation().await.ok();
		}
		// otherwise, the event was sent by an admin so we join the room
		else {
			match room.accept_invitation().await {
				Ok(_) => info!("Successfully joined room {room_id}"),
				Err(err) => error!("Error joining room {room_id}: {err}")
			}
		}
	}
}

async fn send(room: Joined, content: impl MessageLikeEventContent) {
	let room_id = room.room_id();
	match room.send(content, None).await {
		Ok(_) => info!("Sent message to room {room_id}"),
		Err(err) => error!("Error sending message to room {room_id}: {err}")
	}
}

async fn reply(
	room: Joined,
	ev: OriginalSyncRoomMessageEvent,
	content: RoomMessageEventContent
) {
	let room_id = room.room_id().to_owned();
	send(
		room,
		content.make_reply_to(&ev.into_full_event(room_id), ForwardThread::Yes)
	)
	.await;
}

async fn react(room: Joined, ev: OriginalSyncRoomMessageEvent, body: &str) {
	let room_id = room.room_id().to_owned();
	send(
		room,
		ReactionEventContent::new(Annotation::new(ev.event_id, body.to_owned()))
	)
	.await;
}

async fn enqueue_impl(
	room: Joined,
	ev: OriginalSyncRoomMessageEvent,
	job: Job
) -> anyhow::Result<()> {
	let mut q = read_queue(&room.client()).await?;
	q.q.push_back(job);
	write_queue(&room.client(), &q).await?;

	react(room, ev, "👍").await;
	Ok(())
}

async fn enqueue(room: Joined, ev: OriginalSyncRoomMessageEvent, job: Job) {
	match enqueue_impl(room, ev, job).await {
		Ok(_) => info!("Sucessfully enqueued job"),
		Err(err) => error!("Error enqueueing job: {err}")
	}
}

async fn message_handler(ev: OriginalSyncRoomMessageEvent, room: Room, client: Client) {
	// don't reply to our own messages
	if ev.sender == client.user_id().unwrap() {
		return;
	}

	if let Room::Joined(room) = room {
		let MessageType::Text(text_content) = ev.content.msgtype.clone() else {
            return;
        };

		let body = text_content.body.trim_end();
		if !body.starts_with('!') {
			return;
		}

		// help message
		if body == "!help" {
			reply(
				room,
				ev,
				RoomMessageEventContent::text_html(
					indoc! {r#"
						This is tg2mx_bot, a bot that can import sticker packs from
						telegram and migrate maunium's sticker packs to MSC2545 room
						sticker packs.

						The following commands are available:

						!help  --  Show this help message

						!import <pack>  --  Import a telegram sticker pack.

						!migrate <pack>  --  Migrate a maunium sticker pack.
					"#},
					indoc! {r#"
						<p>This is tg2mx_bot, a bot that can import sticker packs from
						telegram and migrate maunium's sticker packs to MSC2545 room
						sticker packs.</p>

						<p>The following commands are available:</p>

						<ul>
						  <li><code>!help</code>  --  Show this help message</li>
						  <li><code>!import</code> &lt;pack&gt;  --  Import a telegram
						      sticker pack.</li>
						  <li><code>!migrate</code> &lt;pack&gt;  --  Migrate a maunium
						      sticker pack.</li>
						</ul>
					"#}
				)
			)
			.await;
		}
		// import tg sticker pack
		else if let Some(pack) = body.strip_prefix("!import ") {
			enqueue(room, ev, Job::Import(pack.to_owned())).await;
		}
		// import maunium sticker pack
		else if let Some(pack) = body.strip_prefix("!migrate ") {
			enqueue(room, ev, Job::Migrate(pack.to_owned())).await;
		}
		// unknown command
		else {
			reply(
				room,
				ev,
				RoomMessageEventContent::text_plain(
					"Unknown command. Use !help to see a list of all commands"
				)
			)
			.await;
		}
	}
}

pub(super) async fn run() -> anyhow::Result<()> {
	let client = Client::builder()
		.homeserver_url(HOMESERVER.as_deref().unwrap())
		.build()
		.await?;
	client
		.login_username(MATRIX_ID.as_deref().unwrap(), PASSWORD.as_deref().unwrap())
		.initial_device_display_name("tg2mx bot")
		.send()
		.await?;
	info!("Logged in successfully");

	// throw away inital sync - this means we don't reply to old messages
	let response = client.sync_once(SyncSettings::default()).await.unwrap();

	// from now on, start handling events
	client.add_event_handler(autojoin_handler);
	client.add_event_handler(message_handler);

	// keep syncing forever
	let sync_fut = async {
		client
			.sync(SyncSettings::default().token(response.next_batch))
			.await?;
		anyhow::Ok(())
	};

	// keep working the queue
	let queue_fut = async {
		loop {
			sleep(Duration::from_secs(1)).await;
			let mut q = read_queue(&client).await?;
			if let Some(job) = q.q.pop_front() {
				write_queue(&client, &q).await?;

				let res = match &job {
					Job::Import(pack) => import(&client, pack).await,
					Job::Migrate(pack) => migrate(&client, pack).await
				};
				if let Err(err) = res {
					error!("Failed to run queued job {job:?}: {err}");
					let mut q = read_queue(&client).await?;
					q.q.push_back(job);
					write_queue(&client, &q).await?;
				}
			}
		}

		// no, this is necessary so that all futures have the same return type
		#[allow(unreachable_code)]
		anyhow::Ok(())
	};

	select(Box::pin(sync_fut), Box::pin(queue_fut))
		.await
		.factor_first()
		.0?;
	Ok(())
}
