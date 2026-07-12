use axum::extract::State;
use conduwuit::{Result, utils::stream::BroadbandExt};
use futures::StreamExt;
use ruma::api::client::user_directory::search_users::{self};

use crate::Ruma;

// conduwuit can handle a lot more results than synapse
const LIMIT_MAX: usize = 500;
const LIMIT_DEFAULT: usize = 10;

/// # `POST /_matrix/client/r0/user_directory/search`
///
/// Searches all known local users for a match against Matrix ID or display name.
pub(crate) async fn search_users_route(
	State(services): State<crate::State>,
	body: Ruma<search_users::v3::Request>,
) -> Result<search_users::v3::Response> {
	let limit = usize::try_from(body.limit)
		.map_or(LIMIT_DEFAULT, usize::from)
		.min(LIMIT_MAX);

	let search_term = body.search_term.to_lowercase();
	let mut users = services
		.users
		.stream()
		.map(ToOwned::to_owned)
		.broad_filter_map(async |user_id| {
			let display_name = services.users.displayname(&user_id).await.ok();

			let user_id_matches = user_id.as_str().to_lowercase().contains(&search_term);

			let display_name_matches = display_name
				.as_deref()
				.map(str::to_lowercase)
				.is_some_and(|display_name| display_name.contains(&search_term));

			if !user_id_matches && !display_name_matches {
				return None;
			}

			Some(search_users::v3::User {
				user_id: user_id.clone(),
				display_name,
				avatar_url: services.users.avatar_url(&user_id).await.ok(),
			})
		});

	let results = users.by_ref().take(limit).collect().await;
	let limited = users.next().await.is_some();

	Ok(search_users::v3::Response { results, limited })
}
