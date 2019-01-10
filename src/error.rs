use actix_web::{HttpResponse, http::StatusCode, error::ResponseError};
use failure::Fail;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "A bad SHA2-256 hash was provided")]
    HashError,
    #[fail(
        display = "A local IPFS API could not be found, and the public API cannot support this functionality"
    )]
    LocalApiUnavailableError,
    #[fail(display = "An error was encountered in a request to the IPFS API")]
    IpfsApiError(StatusCode),
    #[fail(display = "The requested transfer is unavailable, only basic transfer is supported at this time")]
    TransferUnavailable,
    #[fail(display = "An internal server error occurred while serializing data to a json.")]
    SerializeJsonError
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match *self {
            Error::HashError => HttpResponse::new(StatusCode::BAD_REQUEST),
            Error::LocalApiUnavailableError => HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
            Error::IpfsApiError(status) => HttpResponse::new(status),
            Error::TransferUnavailable => HttpResponse::new(StatusCode::NOT_IMPLEMENTED),
            Error::SerializeJsonError => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
