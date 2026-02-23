//! Database handle wrapper

use surrealdb::{
    engine::local::Db,
    Surreal,
};

/// Wrapper around the SurrealDB handle.
/// Clone is cheap (Arc internally).
#[derive(Clone)]
pub struct DbHandle {
    pub db: Surreal<Db>,
}

impl PartialEq for DbHandle {
    fn eq(&self, _other: &Self) -> bool {
        true // Single global instance
    }
}
