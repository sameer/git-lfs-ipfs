use actix_web::{
    client::SendRequestError, error::JsonPayloadError, error::PayloadError, error::ResponseError,
    http::StatusCode, HttpResponse,
};
use failure::Fail;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "bad SHA2-256 hash provided")]
    HashError,
    #[fail(
        display = "local IPFS API could not be found, and the public API cannot support this functionality"
    )]
    LocalApiUnavailableError,
    #[fail(display = "error in parsing an IPFS path {}", _0)]
    IpfsPathParseError(crate::spec::ipfs::PathParseError),
    #[fail(
        display = "error in receiving a response from the IPFS API {:?}",
        _0
    )]
    IpfsApiPayloadError(PayloadError),
    #[fail(
        display = "error in receiving a JSON response from the IPFS API {:?}",
        _0
    )]
    IpfsApiJsonPayloadError(JsonPayloadError),
    #[fail(
        display = "error while sending a request to the IPFS API {:?}",
        _0
    )]
    IpfsApiSendRequestError(SendRequestError),
    #[fail(display = "error received from IPFS API {:?}", _0)]
    IpfsApiResponseError(crate::spec::ipfs::Error),
    #[fail(display = "{}", _0)]
    Io(std::io::Error),
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::HashError => HttpResponse::BadRequest().finish(),
            Error::LocalApiUnavailableError => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
            Error::IpfsPathParseError(_) => HttpResponse::BadRequest().finish(),
            Error::IpfsApiPayloadError(payload_error) => payload_error.error_response(),
            Error::IpfsApiJsonPayloadError(json_payload_error) => {
                json_payload_error.error_response()
            }
            Error::IpfsApiSendRequestError(send_request_error) => {
                send_request_error.error_response()
            }
            Error::IpfsApiResponseError(error) => HttpResponse::InternalServerError().json(error),
            Error::Io(io) => HttpResponse::InternalServerError().finish(),
        }
    }
}
