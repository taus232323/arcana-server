use axum::{Router, extract::State, response::IntoResponse, routing::get};

use crate::{WebError, template};

pub(crate) fn build() -> Router<crate::State> {
	Router::new()
		.route("/", get(index))
		.route("/_continuwuity/", get(index))
}

async fn index(State(services): State<crate::State>) -> Result<impl IntoResponse, WebError> {
	template! {
		struct Index<'a> use "index.html.j2" {
			server_name: &'a str,
			first_run: bool,
			client_domain: String,
			android_store_download: Option<String>,
			android_fdroid_download: Option<String>,
			ios_download: Option<String>,
		}
	}

	Ok(Index::new(
		&services,
		services.globals.server_name().as_str(),
		services.firstrun.is_first_run(),
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
