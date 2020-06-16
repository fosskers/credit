use auto_from::From;

/// Errors that occur during Github communication, etc.
#[derive(From)]
pub enum Error {
    Http(isahc::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Other,
}
