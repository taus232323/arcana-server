use axum::{
	Router,
	extract::{Path, State},
	response::IntoResponse,
	routing::get,
};

use crate::{WebError, template};

pub(crate) fn build() -> Router<crate::State> {
	Router::new().route("/invite/{token}", get(invite))
}

async fn invite(
	State(services): State<crate::State>,
	Path(token): Path<String>,
) -> Result<impl IntoResponse, WebError> {
	let Some(token) = services.invites.check_token(&token).await else {
		return Err(WebError::BadRequest("Invalid invite token.".to_owned()));
	};
	let token_string = token.token;
	let inviter = token.info.inviter.to_string();
	let target = token.info.target.to_string();
	let room_id = token.info.room_id.to_string();

	template! {
		struct Invite<'a> use "invite.html.j2" {
			server_name: &'a str,
			token: String,
			inviter: String,
			target: String,
			room_id: String,
			client_domain: String,
			android_store_download: Option<String>,
			android_fdroid_download: Option<String>,
			ios_download: Option<String>
		}
	}

	Ok(Invite::new(
		&services,
		services.globals.server_name().as_str(),
		token_string,
		inviter,
		target,
		room_id,
		services.config.get_client_domain().to_string(),
		services
			.config
			.well_known
			.android_store_download
			.clone()
			.map(|url| url.to_string()),
		services
			.config
			.well_known
			.android_fdroid_download
			.clone()
			.map(|url| url.to_string()),
		services
			.config
			.well_known
			.ios_download
			.clone()
			.map(|url| url.to_string()),
	)
	.into_response())
}
