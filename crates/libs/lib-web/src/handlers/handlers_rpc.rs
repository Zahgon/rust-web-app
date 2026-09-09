use crate::error::Result;
use crate::middleware::mw_auth::CtxW;
use actix_web::{web, HttpMessage, HttpRequest};
use rpc_router::resources_builder;
use serde_json::{json, Value};
use std::sync::Arc;

/// RPC ID and Method Capture
/// Note: This will be injected into the Actix request extensions so that
///       it can be used downstream by the `mw_res_map` for logging and eventual
///       error client JSON-RPC serialization
#[derive(Debug)]
pub struct RpcInfo {
	pub id: Option<Value>,
	pub method: String,
}

pub async fn rpc_actix_handler(
	http_req: HttpRequest,
	rpc_router: web::Data<rpc_router::Router>,
	ctx: CtxW,
	rpc_req: web::Json<Value>,
) -> Result<web::Json<Value>> {
	let ctx = ctx.0;

	// -- Parse and RpcRequest validate the rpc_request
	let rpc_req = rpc_router::RpcRequest::try_from(rpc_req.into_inner())
		.map_err(crate::Error::RpcRequestParsing)?;

	// -- Create the RPC Info
	//    (will be set to the request extensions)
	// Note: We store data in the Actix request extensions so that
	//       we can unpack it in the `mw_res_map` for client-side rendering.
	//       This approach centralizes error handling for the client at the `mw_res_map` module.
	//       Here, add the captured RpcInfo (RPC ID and method) to be used later in the
	//       `mw_res_map` for RequestLineLogging, and eventual JSON-RPC error serialization.
	let rpc_info = RpcInfo {
		id: Some(rpc_req.id.to_value()),
		method: rpc_req.method.clone(),
	};
	http_req.extensions_mut().insert(Arc::new(rpc_info));

	// -- Add the request specific resources
	// Note: Since Ctx is per actix request, we construct additional RPC resources.
	//       These additional resources will be "overlayed" on top of the base router services,
	//       meaning they will take precedence over the base router ones, but won't replace them.
	let additional_resources = resources_builder![ctx].build();

	// -- Exec Rpc Route
	// Note: Error Json response will be generated in the mw_res_map as will other errors.
	let rpc_call_response = rpc_router
		.call_with_resources(rpc_req, additional_resources)
		.await?;

	// -- Build Json Rpc Success Response
	let body_response = json!({
		"jsonrpc": "2.0",
		"id": rpc_call_response.id,
		"result": rpc_call_response.value
	});

	Ok(web::Json(body_response))
}
