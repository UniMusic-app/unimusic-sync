// Iroh types passable via UniFFI
use iroh_blobs::Hash;
use iroh_docs::{AuthorId, DocTicket, NamespaceId};

use std::fmt::Display;
use std::str::FromStr;

macro_rules! uniffiable_wrapper {
    ($in:ident, $out:ident) => {
        impl From<$in> for $out {
            fn from(value: $in) -> Self {
                Self(value)
            }
        }

        impl From<$out> for $in {
            fn from(value: $out) -> Self {
                value.0
            }
        }

        impl Display for $out {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                Display::fmt(&self.0, f)
            }
        }

        uniffi::custom_type!($out, String, {
            lower: |author_id| author_id.to_string(),
            try_lift: |string| Ok($out($in::from_str(&string)?))
        });
    };
}

#[derive(Debug, Clone, Copy)]
pub struct UAuthorId(AuthorId);
uniffiable_wrapper!(AuthorId, UAuthorId);

#[derive(Debug, Clone, Copy)]
pub struct UNamespaceId(NamespaceId);
uniffiable_wrapper!(NamespaceId, UNamespaceId);

#[derive(Debug, Clone, Copy)]
pub struct UHash(Hash);
uniffiable_wrapper!(Hash, UHash);

#[derive(Debug, Clone)]
pub struct UDocTicket(DocTicket);
uniffiable_wrapper!(DocTicket, UDocTicket);
