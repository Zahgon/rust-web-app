//! Request scoped cookie jar.
//!
//! Note: Actix Web exposes the request cookies, but expects the response cookies
//!       to be set on the response itself. Since cookies are read *and* written by
//!       handlers as well as by middleware, a shared per-request jar is inserted in
//!       the request extensions by the `mw_cookie_manager` middleware, which then
//!       writes the jar "delta" back to the response.

use crate::error::{Error, Result};
use actix_web::cookie::{Cookie, CookieJar};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpMessage, HttpRequest};
use std::future::{ready, Ready};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct Cookies(Arc<Mutex<CookieJar>>);

impl Cookies {
	/// Create the jar from the cookies received with the request.
	pub fn from_request_cookies(cookies: Vec<Cookie<'static>>) -> Self {
		let mut jar = CookieJar::new();
		for cookie in cookies {
			jar.add_original(cookie);
		}

		Cookies(Arc::new(Mutex::new(jar)))
	}

	pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
		self.lock().get(name).cloned()
	}

	pub fn add(&self, cookie: Cookie<'static>) {
		self.lock().add(cookie)
	}

	pub fn remove(&self, cookie: Cookie<'static>) {
		self.lock().remove(cookie)
	}

	/// The cookies added/removed during this request, to be written to the response.
	pub fn delta(&self) -> Vec<Cookie<'static>> {
		self.lock().delta().cloned().collect()
	}

	fn lock(&self) -> std::sync::MutexGuard<'_, CookieJar> {
		// Note: The jar is only ever locked for the duration of a single
		//       operation, so a poisoned lock cannot leave it inconsistent.
		self.0.lock().unwrap_or_else(|ex| ex.into_inner())
	}
}

// region:    --- Cookies Extractor

impl FromRequest for Cookies {
	type Error = Error;
	type Future = Ready<Result<Self>>;

	fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		ready(
			req.extensions()
				.get::<Cookies>()
				.cloned()
				.ok_or(Error::CookiesNotInReqExt),
		)
	}
}

// endregion: --- Cookies Extractor
