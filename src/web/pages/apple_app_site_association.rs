use axum::{
	http::{HeaderValue, header},
	response::{IntoResponse, Response},
	routing::get,
	Router,
};

pub(crate) fn build() -> Router<crate::State> {
	Router::new().route(
		"/.well-known/apple-app-site-association",
		get(apple_app_site_association),
	)
}

async fn apple_app_site_association() -> Response {
	const BODY: &str = r#"{"applinks":{"apps":[],"details":[{"appIDs":["7J4U792NQT.io.arcana.arcana"],"paths":["/invite/*"]}]}}"#;

	(
		[(header::CONTENT_TYPE, HeaderValue::from_static("application/json"))],
		BODY,
	)
		.into_response()
}
