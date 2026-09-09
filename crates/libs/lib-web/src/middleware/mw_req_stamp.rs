use crate::error::{Error, Result};
use actix_web::body::MessageBody;
use actix_web::dev::{Payload, ServiceRequest, ServiceResponse};
use actix_web::middleware::Next;
use actix_web::{FromRequest, HttpMessage, HttpRequest};
use lib_utils::time::now_utc;
use std::future::{ready, Ready};
use time::OffsetDateTime;
use tracing::debug;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ReqStamp {
	pub uuid: Uuid,
	pub time_in: OffsetDateTime,
}

pub async fn mw_req_stamp_resolver<B>(
	req: ServiceRequest,
	next: Next<B>,
) -> core::result::Result<ServiceResponse<B>, actix_web::Error>
where
	B: MessageBody,
{
	debug!("{:<12} - mw_req_stamp_resolver", "MIDDLEWARE");

	let time_in = now_utc();
	let uuid = Uuid::new_v4();

	req.extensions_mut().insert(ReqStamp { uuid, time_in });

	next.call(req).await
}

// region:    --- ReqStamp Extractor
impl FromRequest for ReqStamp {
	type Error = Error;
	type Future = Ready<Result<Self>>;

	fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		debug!("{:<12} - ReqStamp", "EXTRACTOR");

		ready(
			req.extensions()
				.get::<ReqStamp>()
				.cloned()
				.ok_or(Error::ReqStampNotInReqExt),
		)
	}
}
// endregion: --- ReqStamp Extractor
