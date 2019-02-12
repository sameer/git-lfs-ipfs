use failure::Fail;

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "bad SHA2-256 hash provided")]
    HashError,
    #[fail(display = "{}", _0)]
    Io(std::io::Error),
}
