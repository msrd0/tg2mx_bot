use dotenvy::dotenv;
use log::{error, info};
use matrix_sdk::{
	config::SyncSettings,
	room::Room,
	ruma::events::{room::member::StrippedRoomMemberEvent, AnySyncStateEvent},
	Client
};
use std::env;

fn env(name: &str) -> String {
	env::var(name).unwrap_or_else(|_| panic!("Missing {name} environment variable"))
}

async fn autojoin_handler(ev: StrippedRoomMemberEvent, room: Room, client: Client) {
	if ev.state_key != client.user_id().unwrap() {
		return;
	}

	if let Room::Invited(room) = room {
		let room_id = room.room_id();
		match room.accept_invitation().await {
			Ok(()) => info!("Successfully join room {room_id}"),
			Err(err) => error!("Error joining room {room_id}: {err}")
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

	client.add_event_handler(autojoin_handler);

	client.sync(SyncSettings::new()).await?;
	Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	pretty_env_logger::init_timed();
	dotenv().ok();

	run().await
}
