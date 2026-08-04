use axum::{
	http::{HeaderValue, header},
	response::{IntoResponse, Response},
	routing::get,
	Router,
};

pub(crate) fn build() -> Router<crate::State> {
	Router::new().route("/.well-known/assetlinks.json", get(assetlinks))
}

async fn assetlinks() -> Response {
	const BODY: &str = r#"[{"relation":["delegate_permission/common.handle_all_urls"],"target":{"namespace":"android_app","package_name":"ru.celesteai.arcana","sha256_cert_fingerprints":["43:9E:DB:A5:62:A3:78:62:61:8D:06:CF:FA:A1:50:2E:9F:22:EF:2D:71:31:99:8E:3A:CB:DD:AD:62:9A:50:B4"]}}]"#;

	(
		[(header::CONTENT_TYPE, HeaderValue::from_static("application/json"))],
		BODY,
	)
		.into_response()
}
