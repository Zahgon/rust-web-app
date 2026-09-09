// region:    --- Modules

mod config;
mod error;
mod web;

pub use self::error::{Error, Result};
use config::web_config;

use actix_web::HttpServer;
use lib_core::_dev_utils;
use lib_core::model::ModelManager;
use tracing::info;
use tracing_subscriber::EnvFilter;

// endregion: --- Modules

#[actix_web::main]
async fn main() -> Result<()> {
	tracing_subscriber::fmt()
		.without_time() // For early local development.
		.with_target(false)
		.with_env_filter(EnvFilter::from_default_env())
		.init();

	// -- FOR DEV ONLY
	_dev_utils::init_dev().await;

	let mm = ModelManager::new().await?;

	// region:    --- Start Server
	// Note: For this block, ok to unwrap.
	let server = HttpServer::new(move || web::app(mm.clone(), &web_config().WEB_FOLDER))
		.bind(("127.0.0.1", 8080))
		.unwrap();
	info!("{:<12} - {:?}\n", "LISTENING", server.addrs());
	server.run().await.unwrap();
	// endregion: --- Start Server

	Ok(())
}
