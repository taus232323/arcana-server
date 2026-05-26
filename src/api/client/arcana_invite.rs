use axum::{
	extract::{Path, Query, State},
	http::header::AUTHORIZATION,
	Json,
};
use conduwuit::{Err, Result};
use http::HeaderMap;
use serde::{Deserialize, Serialize};

use super::join_room_by_id_helper;

#[derive(Debug, Deserialize)]
struct ArcanaInviteAcceptQuery {
	access_token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcanaInviteResponse {
	token: String,
	kind: Option<String>,
	inviter: Option<ArcanaInviteIdentityResponse>,
	target: Option<ArcanaInviteIdentityResponse>,
	room: Option<ArcanaInviteRoomResponse>,
	web_url: Option<String>,
	expires_at: Option<String>,
	used: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcanaInviteIdentityResponse {
	user_id: Option<String>,
	display_name: Option<String>,
	avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcanaInviteRoomResponse {
	room_id: Option<String>,
	room_name: Option<String>,
	is_dm: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcanaInviteAcceptResponse {
	room_id: String,
	is_new: Option<bool>,
	open_room: Option<bool>,
}

pub(crate) fn build() -> axum::Router<crate::State> {
	axum::Router::new()
		.route("/api/invite/{token}", axum::routing::get(get_invite))
		.route("/api/invite/{token}/accept", axum::routing::post(accept_invite))
}

pub(crate) async fn get_invite(
	State(services): State<crate::State>,
	Path(token): Path<String>,
) -> Result<Json<ArcanaInviteResponse>> {
	let Some(token) = services.invites.check_token(&token).await else {
		return Err!(Request(NotFound("Invite token not found.")));
	};

	let room_name = services.rooms.state_accessor.get_name(&token.info.room_id).await.ok();
	let inviter_display_name = services.users.displayname(&token.info.inviter).await.ok();
	let inviter_avatar_url = services.users.avatar_url(&token.info.inviter).await.ok();
	let target_display_name = services.users.displayname(&token.info.target).await.ok();
	let target_avatar_url = services.users.avatar_url(&token.info.target).await.ok();

	let mut web_url = services.config.get_client_domain().join("/invite").unwrap();
	web_url
		.path_segments_mut()
		.expect("invite URL should support path segments")
		.push(token.token.as_str());

	Ok(Json(ArcanaInviteResponse {
		token: token.token,
		kind: Some("dm".to_owned()),
		inviter: Some(ArcanaInviteIdentityResponse {
			user_id: Some(token.info.inviter.to_string()),
			display_name: inviter_display_name,
			avatar_url: inviter_avatar_url.map(|url| url.to_string()),
		}),
		target: Some(ArcanaInviteIdentityResponse {
			user_id: Some(token.info.target.to_string()),
			display_name: target_display_name,
			avatar_url: target_avatar_url.map(|url| url.to_string()),
		}),
		room: Some(ArcanaInviteRoomResponse {
			room_id: Some(token.info.room_id.to_string()),
			room_name,
			is_dm: Some(true),
		}),
		web_url: Some(web_url.to_string()),
		expires_at: None,
		used: Some(token.info.used_at.is_some()),
	}))
}

pub(crate) async fn accept_invite(
	State(services): State<crate::State>,
	headers: HeaderMap,
	Path(token): Path<String>,
	Query(query): Query<ArcanaInviteAcceptQuery>,
) -> Result<Json<ArcanaInviteAcceptResponse>> {
	let Some(token) = services.invites.check_token(&token).await else {
		return Err!(Request(NotFound("Invite token not found.")));
	};

	let access_token = query
		.access_token
		.or_else(|| bearer_token(&headers).map(str::to_owned))
		.ok_or_else(|| Err!(Request(MissingToken("Missing access token."))))?;

	let (sender_user, _device_id) = services
		.users
		.find_from_token(&access_token)
		.await?;

	if sender_user != token.info.target {
		return Err!(Request(Forbidden(
			"This invite is not addressed to the authenticated user."
		)));
	}

	let was_joined = services
		.rooms
		.state_cache
		.is_joined(&sender_user, &token.info.room_id)
		.await;

	if !was_joined {
		join_room_by_id_helper(
			&services,
			&sender_user,
			&token.info.room_id,
			Some("Accepted invite".to_owned()),
			&[],
			&None,
		)
		.await?;
	}

	services.invites.mark_token_used(&token.token).await;

	Ok(Json(ArcanaInviteAcceptResponse {
		room_id: token.info.room_id.to_string(),
		is_new: Some(!was_joined),
		open_room: Some(true),
	}))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
	let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
	value.strip_prefix("Bearer ")
}
