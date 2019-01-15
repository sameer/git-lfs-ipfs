use actix_web::{
    client::SendRequestError, error::JsonPayloadError, error::PayloadError, error::ResponseError,
    http::StatusCode, HttpResponse,
};
use failure::Fail;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "A bad SHA2-256 hash was provided")]
    HashError,
    #[fail(
        display = "A local IPFS API could not be found, and the public API cannot support this functionality"
    )]
    LocalApiUnavailableError,
    #[fail(display = "An error was encountered in parsing an IPFS path")]
    IpfsPathParseError,
    #[fail(display = "An error was encountered in receiving a response from the IPFS API")]
    IpfsApiPayloadError(PayloadError),
    #[fail(display = "An error was encountered in receiving a JSON response from the IPFS API")]
    IpfsApiJsonPayloadError(JsonPayloadError),
    #[fail(display = "An error was encountered while sending a request to the IPFS API")]
    IpfsApiSendRequestError(SendRequestError),
    #[fail(display = "An error was encountered with a to the IPFS API")]
    IpfsApiResponseError(StatusCode),
    #[fail(
        display = "An object upload is impossible with your current configuration. You must use IPNS and have the matching key available locally."
    )]
    IpfsUploadNotPossible,
    #[fail(
        display = "The requested transfer is unavailable, only basic transfer is supported at this time"
    )]
    TransferUnavailable,
    #[fail(display = "An internal server error occurred while serializing data to a json.")]
    SerializeJsonError,
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::HashError => HttpResponse::new(StatusCode::BAD_REQUEST),
            Error::LocalApiUnavailableError => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
            Error::IpfsPathParseError => HttpResponse::new(StatusCode::BAD_REQUEST),
            Error::IpfsApiPayloadError(payload_error) => payload_error.error_response(),
            Error::IpfsApiJsonPayloadError(json_payload_error) => {
                json_payload_error.error_response()
            }
            Error::IpfsApiSendRequestError(send_request_error) => {
                send_request_error.error_response()
            }
            Error::IpfsApiResponseError(status) => HttpResponse::new(*status),
            Error::IpfsUploadNotPossible => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
            Error::TransferUnavailable => HttpResponse::new(StatusCode::NOT_IMPLEMENTED),
            Error::SerializeJsonError => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
