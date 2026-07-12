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
	const BODY: &str = r#"[{"relation":["delegate_permission/common.handle_all_urls"],"target":{"namespace":"android_app","package_name":"ru.celesteai.arcana","sha256_cert_fingerprints":["B0:B0:51:DC:56:5C:81:2F:E1:7F:6F:3E:94:5B:4D:79:04:71:23:AB:0D:A6:12:86:76:9E:B2:94:91:97:13:0E"]}}]"#;

	(
		[(header::CONTENT_TYPE, HeaderValue::from_static("application/json"))],
		BODY,
	)
		.into_response()
}
