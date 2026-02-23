//! Transfer operations API

use crate::api::{KipError, ScanResult};
use crate::db::DbHandle;

/// Scan an intent's source (re-exported from intent module)
pub async fn scan_intent(db: &DbHandle, intent_id: &str) -> Result<ScanResult, KipError> {
    // This is handled in the intent module
    crate::api::intent::scan_intent(db, intent_id).await
}
