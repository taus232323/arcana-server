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
	template! {
		struct Invite<'a> use "invite.html.j2" {
			server_name: &'a str,
			token: String,
			client_domain: String,
			android_fdroid_download: Option<String>,
			android_gdroid_download: Option<String>,
			ios_download: Option<String>,
		}
	}

	Ok(Invite::new(
		&services,
		services.globals.server_name().as_str(),
		token,
		services.config.get_client_domain().to_string(),
		services.config
			.well_known
			android_fdroid_download
			.clone()
			.map(|url| url.to_string()),
		services.config
			.well_known
			android_gdroid_download
			.clone()
			.map(|url| url.to_string()),
		services
			.config
			.well_known
			ios_download
			.clone()
			.map(|url| url.to_string()),
	)
	.into_response())
}
