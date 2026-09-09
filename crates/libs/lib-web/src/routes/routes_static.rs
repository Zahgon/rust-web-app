use actix_files::Files;
use actix_web::dev::{fn_service, HttpServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::HttpResponse;

// Note: `Files` is a service factory, so it can be registered as the last
//       service of the app and act as the catch-all for the other requests.
pub fn serve_dir(web_folder: &'static String) -> impl HttpServiceFactory {
	Files::new("/", web_folder)
		.index_file("index.html")
		.default_handler(fn_service(|req: ServiceRequest| async move {
			let (req, _) = req.into_parts();
			let res = HttpResponse::build(StatusCode::NOT_FOUND)
				.content_type("text/plain; charset=utf-8")
				.body("Resource not found.");

			Ok(ServiceResponse::new(req, res))
		}))
}
