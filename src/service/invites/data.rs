use std::{
	sync::Arc,
	time::SystemTime,
};

use conduwuit::utils::{ReadyExt, stream::TryExpect};
use database::{Database, Deserialized, Json, Map};
use ruma::{OwnedRoomId, OwnedUserId};
use serde::{Deserialize, Serialize};

pub(super) struct Data {
	invitetoken_info: Arc<Map>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InviteTokenInfo {
	pub inviter: OwnedUserId,
	pub room_id: Option<OwnedRoomId>,
	pub issued_at: SystemTime,
	pub used_at: Option<SystemTime>,
}

impl Data {
	pub(super) fn new(db: &Arc<Database>) -> Self {
		Self {
			invitetoken_info: db["invitetoken_info"].clone(),
		}
	}

	pub(super) fn save_token(&self, token: &str, info: &InviteTokenInfo) {
		self.invitetoken_info.raw_put(token, Json(info));
	}

	pub(super) async fn lookup_token_info(&self, token: &str) -> Option<InviteTokenInfo> {
		self.invitetoken_info.get(token).await.deserialized().ok()
	}

	pub(super) async fn find_token_for_room(
		&self,
		room_id: &OwnedRoomId,
	) -> Option<(String, InviteTokenInfo)> {
		self.invitetoken_info
			.stream::<'_, String, InviteTokenInfo>()
			.expect_ok()
			.ready_find(|(_, info)| info.room_id.as_ref() == Some(room_id))
			.await
	}

	pub(super) fn remove_token(&self, token: &str) { self.invitetoken_info.remove(token); }

	pub(super) async fn revoke_token(&self, token: &str) -> Option<InviteTokenInfo> {
		let info = self.lookup_token_info(token).await?;
		self.remove_token(token);
		Some(info)
	}

	pub(super) async fn mark_token_used(
		&self,
		token: &str,
		room_id: Option<OwnedRoomId>,
	) -> Option<InviteTokenInfo> {
		let mut info = self.lookup_token_info(token).await?;
		if info.used_at.is_none() {
			info.used_at = Some(SystemTime::now());
			if info.room_id.is_none() {
				info.room_id = room_id;
			}
			self.save_token(token, &info);
		}
		Some(info)
	}
}
