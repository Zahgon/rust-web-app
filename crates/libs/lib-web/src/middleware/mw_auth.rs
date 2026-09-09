use crate::error::{Error, Result};
use crate::utils::cookies::Cookies;
use crate::utils::token::{set_token_cookie, AUTH_TOKEN};
use actix_web::body::{BoxBody, MessageBody};
use actix_web::cookie::Cookie;
use actix_web::dev::{Payload, ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::{web, FromRequest, HttpMessage, HttpRequest, HttpResponse};
use lib_auth::token::{validate_web_token, Token};
use lib_core::ctx::Ctx;
use lib_core::model::user::{UserBmc, UserForAuth};
use lib_core::model::ModelManager;
use serde::Serialize;
use std::future::{ready, Ready};
use tracing::debug;

pub async fn mw_ctx_require<B>(
	req: ServiceRequest,
	next: Next<B>,
) -> core::result::Result<ServiceResponse<BoxBody>, actix_web::Error>
where
	B: MessageBody + 'static,
{
	let ctx = ctx_from_req(req.request());
	debug!("{:<12} - mw_ctx_require - {ctx:?}", "MIDDLEWARE");

	match ctx {
		Ok(_) => Ok(next.call(req).await?.map_into_boxed_body()),
		// Note: The error is attached to the response, so that `mw_res_map`
		//       can build the client response out of it.
		Err(ex) => {
			let (http_req, _payload) = req.into_parts();
			Ok(ServiceResponse::new(http_req, HttpResponse::from_error(ex)))
		}
	}
}

// IMPORTANT: This resolver must never fail, but rather capture the potential Auth error and put in in the
//            request extension as CtxExtResult.
//            This way it won't prevent downstream middleware to be executed, and will still capture the error
//            for the appropriate middleware (.e.g., mw_ctx_require which forces successful auth) or handler
//            to get the appropriate information.
pub async fn mw_ctx_resolver<B>(
	req: ServiceRequest,
	next: Next<B>,
) -> core::result::Result<ServiceResponse<B>, actix_web::Error>
where
	B: MessageBody,
{
	debug!("{:<12} - mw_ctx_resolve", "MIDDLEWARE");

	// Note: The `ModelManager` is registered as application data (see `web::app`).
	let mm = req
		.app_data::<web::Data<ModelManager>>()
		.expect("ModelManager should be registered as app_data")
		.get_ref()
		.clone();
	let cookies = req
		.extensions()
		.get::<Cookies>()
		.cloned()
		.ok_or(Error::CookiesNotInReqExt)?;

	let ctx_ext_result = ctx_resolve(mm, &cookies).await;

	if ctx_ext_result.is_err()
		&& !matches!(ctx_ext_result, Err(CtxExtError::TokenNotInCookie))
	{
		cookies.remove(Cookie::named(AUTH_TOKEN))
	}

	// Store the ctx_ext_result in the request extension
	// (for Ctx extractor).
	req.extensions_mut().insert(ctx_ext_result);

	next.call(req).await
}

async fn ctx_resolve(mm: ModelManager, cookies: &Cookies) -> CtxExtResult {
	// -- Get Token String
	let token = cookies
		.get(AUTH_TOKEN)
		.map(|c| c.value().to_string())
		.ok_or(CtxExtError::TokenNotInCookie)?;

	// -- Parse Token
	let token: Token = token.parse().map_err(|_| CtxExtError::TokenWrongFormat)?;

	// -- Get UserForAuth
	let user: UserForAuth =
		UserBmc::first_by_username(&Ctx::root_ctx(), &mm, &token.ident)
			.await
			.map_err(|ex| CtxExtError::ModelAccessError(ex.to_string()))?
			.ok_or(CtxExtError::UserNotFound)?;

	// -- Validate Token
	validate_web_token(&token, user.token_salt)
		.map_err(|_| CtxExtError::FailValidate)?;

	// -- Update Token
	set_token_cookie(cookies, &user.username, user.token_salt)
		.map_err(|_| CtxExtError::CannotSetTokenCookie)?;

	// -- Create CtxExtResult
	Ctx::new(user.id)
		.map(CtxW)
		.map_err(|ex| CtxExtError::CtxCreateFail(ex.to_string()))
}

// region:    --- Ctx Extractor
#[derive(Debug, Clone)]
pub struct CtxW(pub Ctx);

/// Get the `CtxW` resolved by `mw_ctx_resolver` from the request extensions.
pub fn ctx_from_req(req: &HttpRequest) -> Result<CtxW> {
	req.extensions()
		.get::<CtxExtResult>()
		.ok_or(Error::CtxExt(CtxExtError::CtxNotInRequestExt))?
		.clone()
		.map_err(Error::CtxExt)
}

impl FromRequest for CtxW {
	type Error = Error;
	type Future = Ready<Result<Self>>;

	fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		debug!("{:<12} - Ctx", "EXTRACTOR");

		ready(ctx_from_req(req))
	}
}
// endregion: --- Ctx Extractor

// region:    --- Ctx Extractor Result/Error
type CtxExtResult = core::result::Result<CtxW, CtxExtError>;

#[derive(Clone, Serialize, Debug)]
pub enum CtxExtError {
	TokenNotInCookie,
	TokenWrongFormat,

	UserNotFound,
	ModelAccessError(String),
	FailValidate,
	CannotSetTokenCookie,

	CtxNotInRequestExt,
	CtxCreateFail(String),
}
// endregion: --- Ctx Extractor Result/Error
