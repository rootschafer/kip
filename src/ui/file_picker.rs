use std::path::{Path, PathBuf};

use dioxus::prelude::*;
use tracing::{error, info, warn};

use crate::db::DbHandle;

// ─── Pane ID generator ──────────────────────────────────────

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn next_pane_id() -> u64 {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

// ─── Data types ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickerColumn {
    pub dir_path: PathBuf,
    pub entries: Vec<FsEntry>,
    pub selected: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct PickerPaneData {
    pub id: u64,
    pub container_id: String,
    pub container_name: String,
    pub root_path: PathBuf,
    pub columns: Vec<PickerColumn>,
    pub minimized: bool,
    pub show_hidden: bool,
}

// ─── Shared state (provided as context) ─────────────────────

#[derive(Clone, Copy)]
pub struct PickerManager(pub Signal<Vec<PickerPaneData>>);

impl PickerManager {
    pub fn new() -> Self {
        Self(Signal::new(Vec::new()))
    }

    pub fn open(&mut self, container_id: String, container_name: String, root: PathBuf) {
        let mut panes = self.0.write();
        // If pane exists for this container, restore it
        if let Some(pane) = panes.iter_mut().find(|p| p.container_id == container_id) {
            pane.minimized = false;
            return;
        }
        info!("opening picker for {} at {:?}", container_name, root);
        panes.push(PickerPaneData {
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
        self.0.write().retain(|p| p.id != id);
    }

    pub fn minimize(&mut self, id: u64) {
        if let Some(p) = self.0.write().iter_mut().find(|p| p.id == id) {
            p.minimized = true;
        }
    }

    pub fn restore(&mut self, id: u64) {
        if let Some(p) = self.0.write().iter_mut().find(|p| p.id == id) {
            p.minimized = false;
        }
    }

    pub fn has_any(&self) -> bool {
        !self.0.read().is_empty()
    }
}

// ─── Directory reading ──────────────────────────────────────

async fn read_dir_sorted(path: &Path, show_hidden: bool) -> Vec<FsEntry> {
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut entries = Vec::new();
        let iter = match std::fs::read_dir(&path) {
            Ok(it) => it,
            Err(e) => {
                warn!("read_dir {:?}: {}", path, e);
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
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        entries
    })
    .await
    .unwrap_or_default()
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Get the "selected path" from the deepest column that has a selection.
fn selected_path(columns: &[PickerColumn]) -> Option<PathBuf> {
    for col in columns.iter().rev() {
        if let Some(idx) = col.selected {
            if let Some(entry) = col.entries.get(idx) {
                return Some(entry.path.clone());
            }
        }
    }
    None
}

/// Short label for a path (last 1-2 components).
fn short_label(path: &Path) -> String {
    let parts: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    if parts.len() <= 2 {
        path.to_string_lossy().to_string()
    } else {
        parts[parts.len() - 2..].join("/")
    }
}

// ─── Top-level layer ────────────────────────────────────────

#[component]
pub fn FilePickerLayer(on_location_added: EventHandler) -> Element {
    let picker = use_context::<PickerManager>();
    let panes = picker.0.read().clone();

    if panes.is_empty() {
        return rsx! {};
    }

    let has_minimized = panes.iter().any(|p| p.minimized);

    rsx! {
        // Open panes
        for pane in panes.iter().filter(|p| !p.minimized) {
            PickerPaneView {
                key: "{pane.id}",
                pane_id: pane.id,
                on_location_added: on_location_added,
            }
        }

        // Minimized tab bar
        if has_minimized {
            div { class: "picker-tab-bar",
                for pane in panes.iter().filter(|p| p.minimized) {
                    {
                        let id = pane.id;
                        let label = short_label(&pane.root_path);
                        let name = pane.container_name.clone();
                        rsx! {
                            div {
                                key: "{id}",
                                class: "picker-tab",
                                onclick: move |_| {
                                    let mut picker = use_context::<PickerManager>();
                                    picker.restore(id);
                                },
                                oncontextmenu: move |e: Event<MouseData>| {
                                    e.prevent_default();
                                    let mut picker = use_context::<PickerManager>();
                                    picker.close(id);
                                },
                                span { class: "picker-tab-name", "{name}" }
                                span { class: "picker-tab-path", "{label}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ─── Single pane ────────────────────────────────────────────

#[component]
fn PickerPaneView(pane_id: u64, on_location_added: EventHandler) -> Element {
    let mut picker = use_context::<PickerManager>();
    let db = use_context::<DbHandle>();

    // Load root dir on mount
    use_effect(move || {
        let needs_load = {
            let panes = picker.0.read();
            panes
                .iter()
                .find(|p| p.id == pane_id)
                .map(|p| p.columns.is_empty())
                .unwrap_or(false)
        };
        if needs_load {
            let root = {
                let panes = picker.0.read();
                panes
                    .iter()
                    .find(|p| p.id == pane_id)
                    .map(|p| (p.root_path.clone(), p.show_hidden))
            };
            if let Some((root, show_hidden)) = root {
                spawn(async move {
                    let entries = read_dir_sorted(&root, show_hidden).await;
                    let mut panes = picker.0.write();
                    if let Some(p) = panes.iter_mut().find(|p| p.id == pane_id) {
                        p.columns = vec![PickerColumn {
                            dir_path: root,
                            entries,
                            selected: None,
                        }];
                    }
                });
            }
        }
    });

    // Read pane state
    let panes = picker.0.read();
    let Some(pane) = panes.iter().find(|p| p.id == pane_id) else {
        return rsx! {};
    };

    let container_name = pane.container_name.clone();
    let container_id = pane.container_id.clone();
    let columns = pane.columns.clone();
    let show_hidden = pane.show_hidden;

    // Compute selected path for the bottom bar
    let sel_path = selected_path(&columns);
    let sel_display = sel_path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let has_selection = sel_path.is_some();

    // Breadcrumb: show the path of the last column
    let breadcrumb = columns
        .last()
        .map(|c| c.dir_path.to_string_lossy().to_string())
        .unwrap_or_default();

    rsx! {
        div {
            class: "picker-pane",
            onclick: move |e: MouseEvent| e.stop_propagation(),

            // Title bar
            div { class: "picker-title-bar",
                span { class: "picker-title", "{container_name}" }
                div { class: "picker-title-actions",
                    // Toggle hidden files
                    button {
                        class: if show_hidden { "picker-btn-toggle active" } else { "picker-btn-toggle" },
                        title: "Toggle hidden files",
                        onclick: move |_| {
                            let show = {
                                let mut panes = picker.0.write();
                                if let Some(p) = panes.iter_mut().find(|p| p.id == pane_id) {
                                    p.show_hidden = !p.show_hidden;
                                    p.columns.clear();
                                    p.show_hidden
                                } else {
                                    false
                                }
                            };
                            // Force reload by clearing columns — use_effect will reload
                            let _ = show;
                        },
                        ".*"
                    }
                    button {
                        class: "picker-btn-minimize",
                        onclick: move |_| picker.minimize(pane_id),
                        "\u{2212}" // minus sign
                    }
                    button {
                        class: "picker-btn-close",
                        onclick: move |_| picker.close(pane_id),
                        "\u{00d7}" // multiplication sign (×)
                    }
                }
            }

            // Breadcrumb
            div { class: "picker-breadcrumb", "{breadcrumb}" }

            // Column view
            div { class: "picker-columns",
                for (col_idx, col) in columns.iter().enumerate() {
                    div {
                        key: "{col_idx}",
                        class: "picker-column",
                        for (entry_idx, entry) in col.entries.iter().enumerate() {
                            {
                                let is_selected = col.selected == Some(entry_idx);
                                let is_dir = entry.is_dir;
                                let entry_path = entry.path.clone();
                                let name = entry.name.clone();
                                let entry_class = if is_selected {
                                    "picker-entry selected"
                                } else {
                                    "picker-entry"
                                };
                                let size_str = if is_dir {
                                    String::new()
                                } else {
                                    format_size(entry.size)
                                };

                                rsx! {
                                    div {
                                        key: "{name}",
                                        class: "{entry_class}",
                                        onclick: {
                                            let entry_path = entry_path.clone();
                                            move |_| {
                                                let entry_path = entry_path.clone();
                                                spawn(async move {
                                                    // Update selection + truncate
                                                    let show_hidden = {
                                                        let mut panes = picker.0.write();
                                                        let Some(p) = panes.iter_mut().find(|p| p.id == pane_id) else { return };
                                                        p.columns.truncate(col_idx + 1);
                                                        if let Some(col) = p.columns.get_mut(col_idx) {
                                                            col.selected = Some(entry_idx);
                                                        }
                                                        p.show_hidden
                                                    };

                                                    // If dir, load next column
                                                    if is_dir {
                                                        let entries = read_dir_sorted(&entry_path, show_hidden).await;
                                                        let mut panes = picker.0.write();
                                                        if let Some(p) = panes.iter_mut().find(|p| p.id == pane_id) {
                                                            p.columns.push(PickerColumn {
                                                                dir_path: entry_path,
                                                                entries,
                                                                selected: None,
                                                            });
                                                        }
                                                    }
                                                });
                                            }
                                        },

                                        if is_dir {
                                            span { class: "entry-icon dir", "\u{25B8}" } // ▸
                                        } else {
                                            span { class: "entry-icon file", "\u{25AB}" } // ▫
                                        }
                                        span { class: "entry-name", "{name}" }
                                        if !is_dir {
                                            span { class: "entry-size", "{size_str}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Bottom bar: selected path + add button
            div { class: "picker-bottom-bar",
                div { class: "picker-selected-path",
                    if has_selection {
                        "{sel_display}"
                    }
                }
                button {
                    class: "btn-primary picker-add-btn",
                    disabled: !has_selection,
                    onclick: {
                        let container_id = container_id.clone();
                        let sel_path = sel_path.clone();
                        move |_| {
                            if let Some(path) = &sel_path {
                                let path_str = path.to_string_lossy().to_string();
                                let container_id = container_id.clone();
                                let db = db.clone();
                                let on_location_added = on_location_added;
                                spawn(async move {
                                    match add_location_from_picker(&db, &container_id, &path_str).await {
                                        Ok(()) => {
                                            info!("location added from picker: {}", path_str);
                                            on_location_added.call(());
                                        }
                                        Err(e) => error!("add location failed: {}", e),
                                    }
                                });
                            }
                        }
                    },
                    "Add to workspace"
                }
            }
        }
    }
}

// ─── DB action ──────────────────────────────────────────────

fn parse_rid(s: &str) -> Option<(&str, &str)> {
    s.split_once(':')
}

async fn add_location_from_picker(
    db: &DbHandle,
    container_id: &str,
    path: &str,
) -> Result<(), String> {
    let (table, key) = parse_rid(container_id).ok_or("Invalid container ID")?;

    let query = format!(
        "LET $container = type::record('{table}', $key);
         CREATE location CONTENT {{
             {table}: $container,
             path: $path,
             available: true,
             created_at: time::now(),
         }}"
    );

    db.db
        .query(&query)
        .bind(("key", key.to_string()))
        .bind(("path", path.to_string()))
        .await
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())?;

    Ok(())
}
