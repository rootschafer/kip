use dioxus::prelude::*;
use std::path::PathBuf;
use std::collections::HashMap;
use tracing::{info, warn};

use crate::ui::graph_types::PickerPaneData;
use crate::ui::file_picker::{next_pane_id, FsEntry, PickerColumn};

// ─── Picker Manager Store ──────────────────────────────────────────────
// Reactive store for managing picker panes

#[derive(Store, Clone, PartialEq)]
pub struct PickerManager {
    pub panes: Vec<PickerPaneData>,
}

impl PickerManager {
    pub fn new() -> Self {
        Self {
            panes: Vec::new(),
        }
    }

    pub fn open(&mut self, container_id: String, container_name: String, root: PathBuf) {
        // If pane exists for this container, restore it
        if let Some(pane) = self.panes.iter_mut().find(|p| p.container_id == container_id) {
            pane.minimized = false;
            return;
        }
        info!("opening picker for {} at {:?}", container_name, root);
        self.panes.push(PickerPaneData {
            id: next_pane_id(),
            container_id,
            container_name,
            root_path: root,
            columns: vec![],
            minimized: false,
            show_hidden: false,
        });
    }

    pub fn close(&mut self, id: u64) {
        self.panes.retain(|p| p.id != id);
    }

    pub fn minimize(&mut self, id: u64) {
        if let Some(p) = self.panes.iter_mut().find(|p| p.id == id) {
            p.minimized = true;
        }
    }

    pub fn restore(&mut self, id: u64) {
        if let Some(p) = self.panes.iter_mut().find(|p| p.id == id) {
            p.minimized = false;
        }
    }

    pub fn has_any(&self) -> bool {
        !self.panes.is_empty()
    }
}

// ─── Helper functions ──────────────────────────────────────────────────

async fn read_dir_sorted(path: &PathBuf, show_hidden: bool) -> Vec<FsEntry> {
    let path = path.clone();
    tokio::task::spawn_blocking(move || {
        let mut entries = Vec::new();
        let iter = match std::fs::read_dir(&path) {
            Ok(it) => it,
            Err(e) => {
                warn!("read_dir {:?}: {}", path.display(), e);
                return entries;
            }
        };
        for entry in iter.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            let name = entry.file_name().to_string_lossy().to_string();
            if !show_hidden && name.starts_with('.') {
                continue;
            }
            entries.push(FsEntry {
                name,
                path: entry.path(),
                is_dir: meta.is_dir(),
                size: meta.len(),
            });
        }
        entries.sort_by(|a, b| {
            // Directories first, then alphabetically
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        entries
    }).await.unwrap_or_default()
}