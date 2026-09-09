use crate::error::Error;
use crate::handlers::handlers_rpc::RpcInfo;
use crate::log::log_request;
use crate::middleware::mw_auth::ctx_from_req;
use crate::middleware::mw_req_stamp::ReqStamp;

use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::{HttpMessage, HttpResponse};
use serde_json::{json, to_value};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

pub async fn mw_reponse_map<B>(
	req: ServiceRequest,
	next: Next<B>,
) -> core::result::Result<ServiceResponse<BoxBody>, actix_web::Error>
where
	B: MessageBody + 'static,
{
	let res = next.call(req).await?;

	debug!("{:<12} - mw_reponse_map", "RES_MAPPER");
	let uuid = Uuid::new_v4();

	// Note: The handlers and the inner middlewares communicate with this response
	//       mapper through the request extensions.
	let http_req = res.request();
	let req_method = http_req.method().clone();
	let uri = http_req.uri().clone();
	let ctx = ctx_from_req(http_req).map(|ctx| ctx.0).ok();
	let rpc_info = http_req.extensions().get::<Arc<RpcInfo>>().cloned();
	let rpc_info = rpc_info.as_deref();
	let req_stamp = http_req.extensions().get::<ReqStamp>().cloned();

	// -- Get the eventual response error.
	let web_error = res.response().error().and_then(|ex| ex.as_error::<Error>());
	let client_status_error = web_error.map(|se| se.client_status_and_error());

	// -- If client error, build the new reponse.
	let error_response =
		client_status_error
			.as_ref()
			.map(|(status_code, client_error)| {
				let client_error = to_value(client_error).ok();
				let message = client_error.as_ref().and_then(|v| v.get("message"));
				let detail = client_error.as_ref().and_then(|v| v.get("detail"));

				let client_error_body = json!({
					"id": rpc_info.as_ref().map(|rpc| rpc.id.clone()),
					"error": {
						"message": message, // Variant name
						"data": {
							"req_uuid": uuid.to_string(),
							"detail": detail
						},
					}
				});

				debug!("CLIENT ERROR BODY:\n{client_error_body}");

				// Build the new response from the client_error_body
				HttpResponse::build(*status_code).json(client_error_body)
			});

	// -- Build and log the server log line.
	let client_error = client_status_error.unzip().1;

	// TODO: Need to hander if log_request fail (but should not fail request)
	if let Some(req_stamp) = req_stamp {
		let _ = log_request(
			req_method,
			uri,
			req_stamp,
			rpc_info,
			ctx,
			web_error,
			client_error,
		)
		.await;
	}

	debug!("\n");

	let res = match error_response {
		Some(error_response) => {
			let (http_req, _) = res.into_parts();
			ServiceResponse::new(http_req, error_response)
		}
		None => res.map_into_boxed_body(),
	};

	Ok(res)
}
