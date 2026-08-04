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
	let has_inviter = !inviter.is_empty();

	template! {
		struct Invite use "invite.html.j2" {
			token: String,
			inviter: String,
			has_inviter: bool,
			client_domain: String,
			android_store_download: Option<String>,
			android_fdroid_download: Option<String>,
			ios_download: Option<String>
		}
	}

	Ok(Invite::new(
		&services,
		token_string,
		inviter,
		has_inviter,
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
