use crate::error::{Error, Result};
use crate::utils::cookies::Cookies;
use crate::utils::token;
use actix_web::web;
use lib_auth::pwd::{self, ContentToHash, SchemeStatus};
use lib_core::ctx::Ctx;
use lib_core::model::user::{UserBmc, UserForLogin};
use lib_core::model::ModelManager;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::debug;

// region:    --- Login
pub async fn api_login_handler(
	mm: web::Data<ModelManager>,
	cookies: Cookies,
	payload: web::Json<LoginPayload>,
) -> Result<web::Json<Value>> {
	debug!("{:<12} - api_login_handler", "HANDLER");

	let LoginPayload {
		username,
		pwd: pwd_clear,
	} = payload.into_inner();
	let mm = mm.get_ref();
	let root_ctx = Ctx::root_ctx();

	// -- Get the user.
	let user: UserForLogin = UserBmc::first_by_username(&root_ctx, mm, &username)
		.await?
		.ok_or(Error::LoginFailUsernameNotFound)?;
	let user_id = user.id;

	// -- Validate the password.
	let Some(pwd) = user.pwd else {
		return Err(Error::LoginFailUserHasNoPwd { user_id });
	};

	let scheme_status = pwd::validate_pwd(
		ContentToHash {
			salt: user.pwd_salt,
			content: pwd_clear.clone(),
		},
		pwd,
	)
	.await
	.map_err(|_| Error::LoginFailPwdNotMatching { user_id })?;

	// -- Update password scheme if needed
	if let SchemeStatus::Outdated = scheme_status {
		debug!("pwd encrypt scheme outdated, upgrading.");
		UserBmc::update_pwd(&root_ctx, mm, user.id, &pwd_clear).await?;
	}

	// -- Set web token.
	token::set_token_cookie(&cookies, &user.username, user.token_salt)?;

	// Create the success body.
	let body = web::Json(json!({
		"result": {
			"success": true
		}
	}));

	Ok(body)
}

#[derive(Debug, Deserialize)]
pub struct LoginPayload {
	username: String,
	pwd: String,
}
// endregion: --- Login

// region:    --- Logoff
pub async fn api_logoff_handler(
	cookies: Cookies,
	payload: web::Json<LogoffPayload>,
) -> Result<web::Json<Value>> {
	debug!("{:<12} - api_logoff_handler", "HANDLER");
	let should_logoff = payload.logoff;

	if should_logoff {
		token::remove_token_cookie(&cookies)?;
	}

	// Create the success body.
	let body = web::Json(json!({
		"result": {
			"logged_off": should_logoff
		}
	}));

	Ok(body)
}

#[derive(Debug, Deserialize)]
pub struct LogoffPayload {
	logoff: bool,
}
// endregion: --- Logoff
