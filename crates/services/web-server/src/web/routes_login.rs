use actix_web::web::{self, ServiceConfig};
use lib_web::handlers::handlers_login;

pub fn routes(cfg: &mut ServiceConfig) {
	cfg.route("/api/login", web::post().to(handlers_login::api_login_handler))
		.route(
			"/api/logoff",
			web::post().to(handlers_login::api_logoff_handler),
		);
}
