use crate::utils::cookies::Cookies;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::{HeaderValue, SET_COOKIE};
use actix_web::middleware::Next;
use actix_web::{Error, HttpMessage};
use tracing::debug;

/// Insert the request scoped `Cookies` jar in the request extensions, and write
/// back the cookies added/removed during the request to the response.
pub async fn mw_cookie_manager<B>(
	req: ServiceRequest,
	next: Next<B>,
) -> Result<ServiceResponse<B>, Error>
where
	B: MessageBody,
{
	debug!("{:<12} - mw_cookie_manager", "MIDDLEWARE");

	let req_cookies = req
		.cookies()
		.map(|cookies| cookies.clone())
		.unwrap_or_default();
	let cookies = Cookies::from_request_cookies(req_cookies);
	req.extensions_mut().insert(cookies.clone());

	let mut res = next.call(req).await?;

	for cookie in cookies.delta() {
		let value = HeaderValue::from_str(&cookie.encoded().to_string())
			.map_err(actix_web::error::ErrorInternalServerError)?;
		res.headers_mut().append(SET_COOKIE, value);
	}

	Ok(res)
}
