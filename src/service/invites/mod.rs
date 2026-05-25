mod data;

use std::{sync::Arc, time::SystemTime};

use conduwuit::{Err, Result, utils};
use data::{Data, InviteTokenInfo};
use ruma::{OwnedRoomId, OwnedUserId};

use crate::{Dep, globals};

pub const INVITE_WEB_PATH: &str = "/invite";
const INVITE_TOKEN_LENGTH: usize = 32;

pub struct Service {
	db: Data,
	services: Services,
}

struct Services {
	globals: Dep<globals::Service>,
}

#[derive(Debug)]
pub struct ValidInviteToken {
	pub token: String,
	pub info: InviteTokenInfo,
}

impl crate::Service for Service {
	fn build(args: crate::Args<'_>) -> Result<Arc<Self>> {
		Ok(Arc::new(Self {
			db: Data::new(args.db),
			services: Services {
				globals: args.depend::<globals::Service>("globals"),
			},
		}))
	}

	fn name(&self) -> &str { crate::service::make_name(std::module_path!()) }
}

impl Service {
	#[must_use]
	pub fn generate_token_string() -> String { utils::random_string(INVITE_TOKEN_LENGTH) }

	pub async fn issue_token(
		&self,
		inviter: OwnedUserId,
		target: OwnedUserId,
		room_id: OwnedRoomId,
	) -> Result<ValidInviteToken> {
		if !self.services.globals.user_is_local(&inviter) {
			return Err!("Cannot issue an invite token for remote inviter {inviter}");
		}

		if !self.services.globals.user_is_local(&target) {
			return Err!("Cannot issue an invite token for remote target {target}");
		}

		if inviter == target {
			return Err!("Cannot issue an invite token for a direct chat with yourself");
		}

		if let Some((existing_token, _)) = self.db.find_token_for_room(&room_id).await {
			self.db.remove_token(&existing_token);
		}

		let token = Self::generate_token_string();
		let info = InviteTokenInfo {
			inviter,
			target,
			room_id,
			issued_at: SystemTime::now(),
			used_at: None,
		};

		self.db.save_token(&token, &info);

		Ok(ValidInviteToken { token, info })
	}

	pub async fn check_token(&self, token: &str) -> Option<ValidInviteToken> {
		self.db
			.lookup_token_info(token)
			.await
			.map(|info| ValidInviteToken { token: token.to_owned(), info })
	}

	pub async fn mark_token_used(&self, token: &str) -> Option<ValidInviteToken> {
		self.db.mark_token_used(token).await.map(|info| ValidInviteToken {
			token: token.to_owned(),
			info,
		})
	}
}
