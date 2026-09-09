use crate::web::rpcs::all_rpc_router_builder;
use actix_web::dev::HttpServiceFactory;
use actix_web::middleware::from_fn;
use actix_web::web;
use lib_core::model::ModelManager;
use lib_web::handlers::handlers_rpc;
use lib_web::middleware::mw_auth::mw_ctx_require;

/// Build the Actix service for '/api/rpc'
/// Note: This will build the `rpc-router::Router` that will be used by the
///       rpc_actix_handler
pub fn routes(mm: ModelManager) -> impl HttpServiceFactory {
	// Build the combined Rpc Router (from `rpc-router` crate)
	let rpc_router = all_rpc_router_builder()
		// Add the common resources for all rpc calls
		.append_resource(mm)
		.build();

	// Build the Actix service for '/api/rpc'
	web::resource("/api/rpc")
		.app_data(web::Data::new(rpc_router))
		.wrap(from_fn(mw_ctx_require))
		.route(web::post().to(handlers_rpc::rpc_actix_handler))
}
