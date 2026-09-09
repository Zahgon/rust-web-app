// region:    --- Modules

pub mod routes_login;
pub mod routes_rpc;
pub mod rpcs;

#[cfg(test)]
mod tests;

use actix_web::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::middleware::from_fn;
use actix_web::{web, App};
use lib_core::model::ModelManager;
use lib_web::middleware::mw_auth::mw_ctx_resolver;
use lib_web::middleware::mw_cookies::mw_cookie_manager;
use lib_web::middleware::mw_req_stamp::mw_req_stamp_resolver;
use lib_web::middleware::mw_res_map::mw_reponse_map;
use lib_web::routes::routes_static;

// endregion: --- Modules

/// Build the full application (services + middleware layers).
///
/// Note: Extracted from `main` so that the exact same application can be
///       exercised by the integration tests.
pub fn app(
	mm: ModelManager,
	web_folder: &'static String,
) -> App<
	impl ServiceFactory<
		ServiceRequest,
		Config = (),
		Response = ServiceResponse<impl MessageBody>,
		Error = actix_web::Error,
		InitError = (),
	>,
> {
	App::new()
		.app_data(web::Data::new(mm.clone()))
		.configure(routes_login::routes)
		.service(routes_rpc::routes(mm))
		// Note: Registered last, so it acts as the fallback for all other requests.
		.service(routes_static::serve_dir(web_folder))
		// Note: Actix runs the middlewares in reverse registration order,
		//       so `mw_req_stamp_resolver` is the outermost one.
		.wrap(from_fn(mw_reponse_map))
		.wrap(from_fn(mw_ctx_resolver))
		.wrap(from_fn(mw_cookie_manager))
		.wrap(from_fn(mw_req_stamp_resolver))
}
