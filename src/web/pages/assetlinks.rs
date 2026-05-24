use axum::{Json, Router, routing::get};

pub(crate) fn build() -> Router<crate::State> {
	Router::new().route("/.well-known/assetlinks.json", get(assetlinks))
}

async fn assetlinks() -> Json<serde_json::Value> {
	Json(serde_json::json!([
		{
			"relation": [
				"delegate_permission/common.handle_all_urls"
			],
			"target": {
				"namespace": "android_app",
				"package_name": "io.element.android.x",
				"sha256_cert_fingerprints": [
					"B0:B0:51:DC:56:5C:81:2F:E1:7F:6F:3E:94:5B:4D:79:04:71:23:AB:0D:A6:12:86:76:9E:B2:94:91:97:13:0E"
				]
			}
		}
	]))
}
