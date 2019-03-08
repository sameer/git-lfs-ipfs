use failure::Fail;

use git_lfs_spec::transfer::custom;

#[derive(Fail, Debug)]
pub enum CliError {
    #[fail(display = "{}", _0)]
    SerdeJsonError(#[cause] serde_json::error::Error),
    #[fail(display = "{}", _0)]
    Io(#[cause] std::io::Error),
    #[fail(display = "Input was an unexpected event {:?}", _0)]
    UnexpectedEvent(custom::Event),
    #[fail(display = "Error with a request to the IPFS API {:?}", _0)]
    IpfsApiError(ipfs_api::response::Error),
}
