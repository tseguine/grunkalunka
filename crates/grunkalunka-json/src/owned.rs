//! Opt-in RAII ownership for deeply nested JSON trees. The public JsonValue
//! enum remains directly destructurable by value, so it cannot implement
//! Drop without breaking its existing variant-pattern API.
use crate::JsonValue;
use std::ops::{Deref, DerefMut};

/// Releases an owned JSON tree iteratively when the wrapper goes out of scope.
///
/// JSON parsed through this library is bounded at 128 container levels. This
/// wrapper also protects callers assembling or retaining deeper trees, without
/// changing JsonValue's public by-value pattern matching behavior.
#[derive(Clone, Debug)]
pub struct OwnedJson(Option<JsonValue>);

impl OwnedJson {
    pub fn new(value: JsonValue) -> Self {
        Self(Some(value))
    }

    /// Returns the raw enum for ordinary by-value matching. Ownership then
    /// belongs to the caller, including selecting iterative destruction where
    /// the tree may exceed its normal depth limit.
    pub fn into_inner(mut self) -> JsonValue {
        self.0.take().expect("owned JSON value consumed only once")
    }
}

impl From<JsonValue> for OwnedJson {
    fn from(value: JsonValue) -> Self {
        Self::new(value)
    }
}

impl Deref for OwnedJson {
    type Target = JsonValue;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("owned JSON value")
    }
}

impl DerefMut for OwnedJson {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("owned JSON value")
    }
}

impl Drop for OwnedJson {
    fn drop(&mut self) {
        if let Some(value) = self.0.take() {
            value.drop_iteratively();
        }
    }
}
