//! Web layer integration tests.
//!
//! Those tests exercise the whole application (routes, extractors, middlewares,
//! error mapping and static file serving) exactly as it is assembled by
//! `web::app`, against the local dev database.

pub type Result<T> = core::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error>; // For tests.

use crate::web::app;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::http::header::{COOKIE, SET_COOKIE};
use actix_web::http::StatusCode;
use actix_web::{test, Error as ActixError};
use lib_core::_dev_utils;
use lib_core::model::ModelManager;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::OnceLock;

// region:    --- Test Support

const AUTH_TOKEN: &str = "auth-token";

/// The `web-folder` as an absolute path.
/// Note: `cargo test` current dir is the crate dir, not the workspace dir,
///       so the `SERVICE_WEB_FOLDER` relative path cannot be used here.
fn web_folder() -> &'static String {
	static INSTANCE: OnceLock<String> = OnceLock::new();

	INSTANCE.get_or_init(|| {
		Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("../../../web-folder")
			.to_string_lossy()
			.to_string()
	})
}

/// Minimal http test client which keeps the `auth-token` cookie between calls.
struct TestApp<S> {
	service: S,
	cookie: Option<String>,
}

impl TestApp<()> {
	async fn new() -> Result<
		TestApp<
			impl Service<
				actix_http::Request,
				Response = ServiceResponse<impl MessageBody>,
				Error = ActixError,
			>,
		>,
	> {
		// Make sure the dev database is created and seeded.
		_dev_utils::init_test().await;

		// Note: A `ModelManager` per test, since its db pool is bound to the
		//       tokio runtime of the test that created it.
		let mm = ModelManager::new().await?;

		Ok(TestApp {
			service: test::init_service(app(mm, web_folder())).await,
			cookie: None,
		})
	}
}

impl<S, B> TestApp<S>
where
	S: Service<actix_http::Request, Response = ServiceResponse<B>, Error = ActixError>,
	B: MessageBody,
{
	/// Send the request, capture the eventual `auth-token` cookie, and
	/// return the status with the raw body.
	async fn send(
		&mut self,
		req: actix_http::Request,
	) -> Result<(StatusCode, Vec<u8>)> {
		let res = test::call_service(&self.service, req).await;

		let status = res.status();

		for value in res.headers().get_all(SET_COOKIE) {
			let value = value.to_str()?;
			if let Some(pair) = value.split(';').next() {
				if pair.starts_with(&format!("{AUTH_TOKEN}=")) {
					self.cookie = Some(pair.to_string());
				}
			}
		}

		let body = test::read_body(res).await.to_vec();

		Ok((status, body))
	}

	fn base_request(&self, uri: &str) -> test::TestRequest {
		let builder = test::TestRequest::default().uri(uri);

		match self.cookie.as_ref() {
			Some(cookie) => builder.insert_header((COOKIE, cookie.as_str())),
			None => builder,
		}
	}

	async fn post(&mut self, uri: &str, body: Value) -> Result<(StatusCode, Value)> {
		let req = self
			.base_request(uri)
			.method(actix_web::http::Method::POST)
			.set_json(body)
			.to_request();

		let (status, body) = self.send(req).await?;

		Ok((status, serde_json::from_slice(&body)?))
	}

	async fn get(&mut self, uri: &str) -> Result<(StatusCode, String)> {
		let req = self
			.base_request(uri)
			.method(actix_web::http::Method::GET)
			.to_request();

		let (status, body) = self.send(req).await?;

		Ok((status, String::from_utf8(body)?))
	}
}

// endregion: --- Test Support

// region:    --- Login/Logoff

#[actix_web::test]
async fn test_web_login_ok() -> Result<()> {
	// -- Setup & Fixtures
	let mut app = TestApp::new().await?;

	// -- Exec
	let (status, body) = app
		.post("/api/login", json!({"username": "demo1", "pwd": "welcome"}))
		.await?;

	// -- Check
	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["result"]["success"], json!(true));
	assert!(app.cookie.is_some(), "the auth-token cookie should be set");

	Ok(())
}

#[actix_web::test]
async fn test_web_login_fail_pwd_not_matching() -> Result<()> {
	// -- Setup & Fixtures
	let mut app = TestApp::new().await?;

	// -- Exec
	let (status, body) = app
		.post("/api/login", json!({"username": "demo1", "pwd": "wrong-pwd"}))
		.await?;

	// -- Check
	assert_eq!(status, StatusCode::FORBIDDEN);
	assert_eq!(body["error"]["message"], json!("LOGIN_FAIL"));
	assert!(app.cookie.is_none(), "no auth-token cookie should be set");

	Ok(())
}

#[actix_web::test]
async fn test_web_login_fail_username_not_found() -> Result<()> {
	// -- Setup & Fixtures
	let mut app = TestApp::new().await?;

	// -- Exec
	let (status, body) = app
		.post(
			"/api/login",
			json!({"username": "no-such-user", "pwd": "welcome"}),
		)
		.await?;

	// -- Check
	assert_eq!(status, StatusCode::FORBIDDEN);
	assert_eq!(body["error"]["message"], json!("LOGIN_FAIL"));

	Ok(())
}

#[actix_web::test]
async fn test_web_logoff_ok() -> Result<()> {
	// -- Setup & Fixtures
	let mut app = TestApp::new().await?;
	app.post("/api/login", json!({"username": "demo1", "pwd": "welcome"}))
		.await?;

	// -- Exec
	let (status, body) = app.post("/api/logoff", json!({"logoff": true})).await?;

	// -- Check
	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["result"]["logged_off"], json!(true));

	Ok(())
}

// endregion: --- Login/Logoff

// region:    --- Rpc

#[actix_web::test]
async fn test_web_rpc_fail_no_auth() -> Result<()> {
	// -- Setup & Fixtures
	let mut app = TestApp::new().await?;

	// -- Exec
	let (status, body) = app
		.post(
			"/api/rpc",
			json!({
				"jsonrpc": "2.0",
				"id": 1,
				"method": "list_agents"
			}),
		)
		.await?;

	// -- Check
	assert_eq!(status, StatusCode::FORBIDDEN);
	assert_eq!(body["error"]["message"], json!("NO_AUTH"));

	Ok(())
}

#[actix_web::test]
async fn test_web_rpc_agent_create_get_delete_ok() -> Result<()> {
	// -- Setup & Fixtures
	let mut app = TestApp::new().await?;
	let fx_name = "test_web_rpc_agent_create_get_delete_ok agent";
	app.post("/api/login", json!({"username": "demo1", "pwd": "welcome"}))
		.await?;

	// -- Exec & Check - create_agent
	let (status, body) = app
		.post(
			"/api/rpc",
			json!({
				"jsonrpc": "2.0",
				"id": 1,
				"method": "create_agent",
				"params": {"data": {"name": fx_name}}
			}),
		)
		.await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["jsonrpc"], json!("2.0"));
	assert_eq!(body["id"], json!(1));
	assert_eq!(body["result"]["data"]["name"], json!(fx_name));
	let agent_id = body["result"]["data"]["id"]
		.as_i64()
		.ok_or("create_agent should return the new agent id")?;

	// -- Exec & Check - get_agent
	let (status, body) = app
		.post(
			"/api/rpc",
			json!({
				"jsonrpc": "2.0",
				"id": 2,
				"method": "get_agent",
				"params": {"id": agent_id}
			}),
		)
		.await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["id"], json!(2));
	assert_eq!(body["result"]["data"]["id"], json!(agent_id));
	assert_eq!(body["result"]["data"]["name"], json!(fx_name));

	// -- Exec & Check - delete_agent (clean)
	let (status, body) = app
		.post(
			"/api/rpc",
			json!({
				"jsonrpc": "2.0",
				"id": 3,
				"method": "delete_agent",
				"params": {"id": agent_id}
			}),
		)
		.await?;
	assert_eq!(status, StatusCode::OK);
	assert_eq!(body["result"]["data"]["id"], json!(agent_id));

	Ok(())
}

#[actix_web::test]
async fn test_web_rpc_fail_method_unknown() -> Result<()> {
	// -- Setup & Fixtures
	let mut app = TestApp::new().await?;
	app.post("/api/login", json!({"username": "demo1", "pwd": "welcome"}))
		.await?;

	// -- Exec
	let (status, body) = app
		.post(
			"/api/rpc",
			json!({
				"jsonrpc": "2.0",
				"id": 1,
				"method": "no_such_method"
			}),
		)
		.await?;

	// -- Check
	assert_eq!(status, StatusCode::BAD_REQUEST);
	assert_eq!(body["id"], json!(1));
	assert_eq!(body["error"]["message"], json!("RPC_REQUEST_METHOD_UNKNOWN"));

	Ok(())
}

#[actix_web::test]
async fn test_web_rpc_fail_params_missing() -> Result<()> {
	// -- Setup & Fixtures
	let mut app = TestApp::new().await?;
	app.post("/api/login", json!({"username": "demo1", "pwd": "welcome"}))
		.await?;

	// -- Exec
	// Note: `create_agent` requires params.
	let (status, body) = app
		.post(
			"/api/rpc",
			json!({
				"jsonrpc": "2.0",
				"id": 1,
				"method": "create_agent"
			}),
		)
		.await?;

	// -- Check
	assert_eq!(status, StatusCode::BAD_REQUEST);
	assert_eq!(body["id"], json!(1));
	assert_eq!(body["error"]["message"], json!("RPC_PARAMS_INVALID"));

	Ok(())
}

// endregion: --- Rpc

// region:    --- Static

#[actix_web::test]
async fn test_web_static_index_ok() -> Result<()> {
	// -- Setup & Fixtures
	let mut app = TestApp::new().await?;

	// -- Exec
	let (status, body) = app.get("/index.html").await?;

	// -- Check
	assert_eq!(status, StatusCode::OK);
	assert!(
		body.contains("Hello <strong>World!</strong>"),
		"should serve the web-folder index.html"
	);

	Ok(())
}

#[actix_web::test]
async fn test_web_static_root_index_ok() -> Result<()> {
	// -- Setup & Fixtures
	let mut app = TestApp::new().await?;

	// -- Exec
	let (status, body) = app.get("/").await?;

	// -- Check
	assert_eq!(status, StatusCode::OK);
	assert!(
		body.contains("Hello <strong>World!</strong>"),
		"the web-folder index.html should be served at the root"
	);

	Ok(())
}

#[actix_web::test]
async fn test_web_static_fail_not_found() -> Result<()> {
	// -- Setup & Fixtures
	let mut app = TestApp::new().await?;

	// -- Exec
	let (status, body) = app.get("/no-such-file.html").await?;

	// -- Check
	assert_eq!(status, StatusCode::NOT_FOUND);
	assert_eq!(body, "Resource not found.");

	Ok(())
}

// endregion: --- Static
