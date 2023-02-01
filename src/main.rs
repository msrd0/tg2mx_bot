use dotenvy::dotenv;
use indoc::indoc;
use log::{error, info};
use matrix_sdk::{
	config::SyncSettings,
	room::{Joined, Room},
	ruma::events::room::{
		member::StrippedRoomMemberEvent,
		message::{
			ForwardThread, MessageType, OriginalSyncRoomMessageEvent,
			RoomMessageEventContent
		}
	},
	Client
};
use std::env;

fn env(name: &str) -> String {
	env::var(name).unwrap_or_else(|_| panic!("Missing {name} environment variable"))
}

async fn autojoin_handler(ev: StrippedRoomMemberEvent, room: Room, client: Client) {
	// ignore member events for other users
	if ev.state_key != client.user_id().unwrap() {
		return;
	}

	if let Room::Invited(room) = room {
		let room_id = room.room_id();
		match room.accept_invitation().await {
			Ok(_) => info!("Successfully joined room {room_id}"),
			Err(err) => error!("Error joining room {room_id}: {err}")
		}
	}
}

async fn send(room: Joined, content: RoomMessageEventContent) {
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
			reply(
				room,
				ev,
				RoomMessageEventContent::text_plain(format!(
					"UNIMPLEMENTED: Import tg sticker pack {pack}"
				))
			)
			.await;
		}
		// import maunium sticker pack
		else if let Some(pack) = body.strip_prefix("!migrate ") {
			reply(
				room,
				ev,
				RoomMessageEventContent::text_plain(format!(
					"UNIMPLEMENTED: Import maunium sticker pack {pack}"
				))
			)
			.await;
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

async fn run() -> anyhow::Result<()> {
	let homeserver = env("HOMESERVER");
	let matrix_id = env("MATRIX_ID");
	let password = env("PASSWORD");

	let client = Client::builder()
		.homeserver_url(&homeserver)
		.build()
		.await?;
	client
		.login_username(&matrix_id, &password)
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
	client
		.sync(SyncSettings::default().token(response.next_batch))
		.await?;
	Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	pretty_env_logger::init_timed();
	dotenv().ok();

	run().await
}
