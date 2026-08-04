use axum::{Router, response::IntoResponse, routing::get};

use crate::{WebError, pages::TemplateContext, template};

pub(crate) fn build() -> Router<crate::State> {
	Router::new()
		.route("/privacy", get(privacy))
		.route("/privacy/", get(privacy))
		.route("/terms", get(terms))
		.route("/terms/", get(terms))
}

async fn privacy() -> Result<impl IntoResponse, WebError> {
	template! {
		struct Privacy use "privacy.html.j2" {}
	}

	Ok(Privacy {
		context: TemplateContext {
			allow_indexing: true,
		},
	})
}

async fn terms() -> Result<impl IntoResponse, WebError> {
	template! {
		struct Terms use "terms.html.j2" {}
	}

	Ok(Terms {
		context: TemplateContext {
			allow_indexing: true,
		},
	})
}
