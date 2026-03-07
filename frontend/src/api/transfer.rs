//! Transfer operations API

use daemon::DbHandle;

use crate::api::{KipError, ScanResult};

/// Scan an intent's source (re-exported from intent module)
pub async fn scan_intent(db: &DbHandle, intent_id: &str) -> Result<ScanResult, KipError> {
	// This is handled in the intent module
	crate::api::intent::scan_intent(db, intent_id).await
}
