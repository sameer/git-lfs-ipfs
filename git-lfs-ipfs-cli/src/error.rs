use git_lfs_spec::transfer::custom;

#[derive(Display, Debug)]
pub enum CliError {
    #[display(fmt = "{}", _0)]
    SerdeJsonError(serde_json::error::Error),
    #[display(fmt = "{}", _0)]
    Io(std::io::Error),
    #[display(fmt = "Input was an unexpected event {:?}", _0)]
    UnexpectedEvent(custom::Event),
    // TODO: Actix developers made the Actix error !Send + !Sync. Once this is handled in rust-ipfs-api, can restore the error here.
    #[display(fmt = "Error with a request to the IPFS API {:?}", _0)]
    IpfsApiError(String),
}
