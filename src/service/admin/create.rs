use std::collections::BTreeMap;

use conduwuit::{Err, Result, info, pdu::PduBuilder};
use futures::{FutureExt, StreamExt};
use ruma::{
	RoomId, RoomVersionId, UserId,
	events::{GlobalAccountDataEventType, direct::DirectEvent},
	events::room::{
		canonical_alias::RoomCanonicalAliasEventContent,
		create::RoomCreateEventContent,
		guest_access::{GuestAccess, RoomGuestAccessEventContent},
		history_visibility::{HistoryVisibility, RoomHistoryVisibilityEventContent},
		join_rules::{JoinRule, RoomJoinRulesEventContent},
		member::{MembershipState, RoomMemberEventContent},
		name::RoomNameEventContent,
		power_levels::RoomPowerLevelsEventContent,
		preview_url::RoomPreviewUrlsEventContent,
		topic::RoomTopicEventContent,
	},
};

use crate::Services;

/// Create the admin room.
///
/// Users in this room are considered admins by conduwuit, and the room can be
/// used to issue admin commands by talking to the server user inside it.
pub async fn create_admin_room(services: &Services) -> Result {
	let room_id = RoomId::new(services.globals.server_name());
	let room_version = &RoomVersionId::V11;

	let _short_id = services
		.rooms
		.short
		.get_or_create_shortroomid(&room_id)
		.await;

	let state_lock = services.rooms.state.mutex.lock(&room_id).await;

	// Create a user for the server
	let server_user = services.globals.server_user.as_ref();
	services.users.create(server_user, None, None).await?;

	let create_content = {
		use RoomVersionId::*;
		match room_version {
			| V1 | V2 | V3 | V4 | V5 | V6 | V7 | V8 | V9 | V10 =>
				RoomCreateEventContent::new_v1(server_user.into()),
			| V11 => RoomCreateEventContent::new_v11(),
			| _ => RoomCreateEventContent::new_v12(),
		}
	};

	info!("Creating admin room {} with version {}", room_id, room_version);

	// 1. The room create event
	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(String::new(), &RoomCreateEventContent {
				federate: true,
				predecessor: None,
				room_version: room_version.clone(),
				..create_content
			}),
			server_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	// 2. Make server user/bot join
	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(
				String::from(server_user),
				&RoomMemberEventContent::new(MembershipState::Join),
			),
			server_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	// 3. Power levels
	let users = BTreeMap::from_iter([(server_user.into(), 69420.into())]);

	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(String::new(), &RoomPowerLevelsEventContent {
				users,
				..Default::default()
			}),
			server_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	// 4.1 Join Rules
	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(String::new(), &RoomJoinRulesEventContent::new(JoinRule::Invite)),
			server_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	// 4.2 History Visibility
	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(
				String::new(),
				&RoomHistoryVisibilityEventContent::new(HistoryVisibility::Shared),
			),
			server_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	// 4.3 Guest Access
	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(
				String::new(),
				&RoomGuestAccessEventContent::new(GuestAccess::Forbidden),
			),
			server_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	// 5. Events implied by name and topic
	let room_name = format!("{} Admin Room", services.config.server_name);
	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(String::new(), &RoomNameEventContent::new(room_name)),
			server_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(String::new(), &RoomTopicEventContent {
				topic: format!("Manage {} | Run commands prefixed with `!admin` | Run `!admin -h` for help | Documentation: https://continuwuity.org/", services.config.server_name),
			}),
			server_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	// 6. Room alias
	let alias = &services.globals.admin_alias;

	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(String::new(), &RoomCanonicalAliasEventContent {
				alias: Some(alias.clone()),
				alt_aliases: Vec::new(),
			}),
			server_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	services
		.rooms
		.alias
		.set_alias(alias, &room_id, server_user)?;

	// 7. (ad-hoc) Disable room URL previews for everyone by default
	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(String::new(), &RoomPreviewUrlsEventContent { disabled: true }),
			server_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	Ok(())
}

pub async fn create_or_get_direct_room(
	services: &Services,
	sender_user: &UserId,
	recipient_user: &UserId,
) -> Result<RoomId> {
	if sender_user == recipient_user {
		return Err!("Cannot create a direct room with yourself");
	}

	if let Ok(direct_event) = services
		.account_data
		.get_global::<DirectEvent>(sender_user, GlobalAccountDataEventType::Direct)
		.await
	{
		if let Some(room_ids) = direct_event
			.content
			.0
			.iter()
			.find(|(user, _)| user.to_string() == recipient_user.as_str())
			.map(|(_, room_ids)| room_ids)
		{
			for room_id in room_ids {
				if services.rooms.state_cache.is_joined(sender_user, room_id).await
					&& services.rooms.state_cache.room_joined_count(room_id).await.unwrap_or(0) <= 2
				{
					info!("Reusing direct room {} for {} and {}", room_id, sender_user, recipient_user);
					return Ok(room_id.to_owned());
				}
			}
		}
	}

	let shared_rooms = services.rooms.state_cache.get_shared_rooms(sender_user, recipient_user);
	futures::pin_mut!(shared_rooms);
	while let Some(room_id) = shared_rooms.next().await {
		if services.rooms.state_cache.room_joined_count(room_id).await.unwrap_or(0) <= 2 {
			info!("Reusing shared room {} for {} and {}", room_id, sender_user, recipient_user);
			ensure_direct_account_data(services, sender_user, recipient_user, room_id).await?;
			ensure_direct_account_data(services, recipient_user, sender_user, room_id).await?;
			return Ok(room_id.to_owned());
		}
	}

	let room_id = RoomId::new(services.globals.server_name());
	let room_version = &services.server.config.default_room_version;

	let _short_id = services
		.rooms
		.short
		.get_or_create_shortroomid(&room_id)
		.await;
	let state_lock = services.rooms.state.mutex.lock(&room_id).await;

	let create_content = {
		use RoomVersionId::*;
		match room_version {
			| V1 | V2 | V3 | V4 | V5 | V6 | V7 | V8 | V9 | V10 =>
				RoomCreateEventContent::new_v1(sender_user.into()),
			| V11 => RoomCreateEventContent::new_v11(),
			| _ => RoomCreateEventContent::new_v12(),
		}
	};

	info!("Creating direct room {} with version {}", room_id, room_version);

	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(String::new(), &RoomCreateEventContent {
				federate: true,
				predecessor: None,
				room_version: room_version.clone(),
				..create_content
			}),
			sender_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(
				String::from(sender_user),
				&RoomMemberEventContent {
					displayname: services.users.displayname(sender_user).await.ok(),
					avatar_url: services.users.avatar_url(sender_user).await.ok(),
					blurhash: services.users.blurhash(sender_user).await.ok(),
					is_direct: Some(true),
					..RoomMemberEventContent::new(MembershipState::Join)
				},
			),
			sender_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(
				String::new(),
				&RoomPowerLevelsEventContent {
					users: BTreeMap::from_iter([(sender_user.to_owned(), 100.into())]),
					..Default::default()
				},
			),
			sender_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(String::new(), &RoomJoinRulesEventContent::new(JoinRule::Invite)),
			sender_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(
				String::new(),
				&RoomHistoryVisibilityEventContent::new(HistoryVisibility::Shared),
			),
			sender_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(
				String::new(),
				&RoomGuestAccessEventContent::new(GuestAccess::Forbidden),
			),
			sender_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	drop(state_lock);

	let state_lock = services.rooms.state.mutex.lock(&room_id).await;
	services
		.rooms
		.timeline
		.build_and_append_pdu(
			PduBuilder::state(
				recipient_user.to_string(),
				&RoomMemberEventContent {
					displayname: services.users.displayname(recipient_user).await.ok(),
					avatar_url: services.users.avatar_url(recipient_user).await.ok(),
					blurhash: services.users.blurhash(recipient_user).await.ok(),
					is_direct: Some(true),
					reason: None,
					..RoomMemberEventContent::new(MembershipState::Invite)
				},
			),
			sender_user,
			Some(&room_id),
			&state_lock,
		)
		.boxed()
		.await?;

	ensure_direct_account_data(services, sender_user, recipient_user, &room_id).await?;
	ensure_direct_account_data(services, recipient_user, sender_user, &room_id).await?;

	info!("Created direct room {} for {} and {}", room_id, sender_user, recipient_user);

	Ok(room_id)
}

async fn ensure_direct_account_data(
	services: &Services,
	user_id: &UserId,
	peer_user: &UserId,
	room_id: &RoomId,
) -> Result<()> {
	let mut direct_rooms = services
		.account_data
		.get_global::<DirectEvent>(user_id, GlobalAccountDataEventType::Direct)
		.await
		.map(|event| event.content.0)
		.unwrap_or_default();

	let room_ids = direct_rooms.entry(peer_user.to_owned()).or_default();
	if !room_ids.iter().any(|existing| existing == room_id) {
		room_ids.push(room_id.to_owned());
	}

	let data = serde_json::json!({
		"type": "m.direct",
		"content": direct_rooms,
	});

	services
		.account_data
		.update(
			None,
			user_id,
			GlobalAccountDataEventType::Direct.to_string().into(),
			&data,
		)
		.await
}
