//! A trait to implement features that currently only exist in Rust Nightly.

/// Flowing into an `Option`. Intended for `bool`.
pub trait Someable {
    fn bool_some<T>(self, t: T) -> Option<T>;

    // fn then<T, F>(self, f: F) -> Option<T>
    // where
    //     F: FnOnce() -> T;
}

impl Someable for bool {
    fn bool_some<T>(self, t: T) -> Option<T> {
        if self {
            Some(t)
        } else {
            None
        }
    }
}
