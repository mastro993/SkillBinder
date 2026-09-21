//! The SQL behind each body of local state, one module per body.
//!
//! Every function takes the caller's connection and fails with `diesel::result::Error`; the JSON
//! payload columns are serialized and parsed by `crate::StateStore`, so the repositories speak in
//! text and rows only.

pub(crate) mod observations;
pub(crate) mod onboarding;
pub(crate) mod plans;
pub(crate) mod roots;

pub(crate) use observations::ObservationRow;
pub(crate) use roots::DUPLICATE_PATH_MESSAGE;
