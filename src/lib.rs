#![deny(clippy::all)]

mod absolute;
mod errors;
mod relative;

pub use absolute::AbsolutePath;
pub use absolute::AbsolutePathBuf;
pub use errors::*;
pub use relative::RelativePath;
pub use relative::RelativePathBuf;

// Absolute + Relative
// Serialize / Deserialize
// Buf / NonBuf versions
// Display
// db serializable
// Normalize
// Check "join" is relative for RelativePath

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
