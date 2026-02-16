use std::path::{Path, PathBuf};

use dioxus::prelude::*;
use tracing::{error, info, warn};

use crate::db::DbHandle;

// ─── Pane ID generator ──────────────────────────────────────

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
pub fn next_pane_id() -> u64 {
	NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

// ─── Data types ─────────────────────────────────────────────

#[derive(Store, Debug, Clone, PartialEq)]
pub struct FsEntry {
	pub name: String,
	pub path: PathBuf,
	pub is_dir: bool,
	pub size: u64,
}

#[derive(Store, Debug, Clone, PartialEq)]
pub struct PickerColumn {
	pub dir_path: PathBuf,
	pub entries: Vec<FsEntry>,
	pub selected: Option<usize>,
}

#[derive(Store, Debug, Clone, PartialEq)]
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

#[derive(Store, Clone, PartialEq)]
pub struct PickerManager {
	pub panes: Vec<PickerPaneData>,
}

#[store(pub)]
impl Store<PickerManager> {
	fn open(&mut self, container_id: String, container_name: String, root: PathBuf) {
		let panes = self.panes();
		// If pane exists for this container, restore it
		{
			let panes_read = panes.read();
			if let Some(idx) = panes_read
				.iter()
				.position(|p| p.container_id == container_id)
			{
				drop(panes_read);
				panes.index(idx).minimized().set(false);
				return;
			}
		}
		info!("opening picker for {} at {:?}", container_name, root);
		self.panes().push(PickerPaneData {
			id: next_pane_id(),
			container_id,
			container_name,
			root_path: root,
			columns: vec![],
			minimized: false,
			show_hidden: false,
		});
	}

	fn close(&mut self, id: u64) {
		self.panes().retain(|p| p.id != id);
	}

	fn minimize(&mut self, id: u64) {
		let panes = self.panes();
		let panes_read = panes.read();
		if let Some(idx) = panes_read.iter().position(|p| p.id == id) {
			drop(panes_read);
			panes.index(idx).minimized().set(true);
		}
	}

	fn restore(&mut self, id: u64) {
		let panes = self.panes();
		let panes_read = panes.read();
		if let Some(idx) = panes_read.iter().position(|p| p.id == id) {
			drop(panes_read);
			panes.index(idx).minimized().set(false);
		}
	}

	fn has_any(&self) -> bool {
		!self.panes().is_empty()
	}
}

impl PickerManager {
	pub fn new() -> Self {
		Self { panes: vec![] }
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
pub fn FilePickerLayer(picker: Store<PickerManager>, on_location_added: EventHandler) -> Element {
	if !picker.has_any() {
		return rsx! {};
	}

	// Collect pane data snapshots for rendering (avoid holding borrows across RSX)
	let panes_store = picker.panes();
	let panes_snapshot = panes_store.cloned();

	let open_pane_ids: Vec<u64> = panes_snapshot
		.iter()
		.filter(|p| !p.minimized)
		.map(|p| p.id)
		.collect();

	let minimized_panes: Vec<(u64, String, String)> = panes_snapshot
		.iter()
		.filter(|p| p.minimized)
		.map(|p| (p.id, short_label(&p.root_path), p.container_name.clone()))
		.collect();

	let has_minimized = !minimized_panes.is_empty();

	rsx! {
		// Open panes
		for pane_id in open_pane_ids.iter() {
			PickerPaneView {
				picker,
				key: "{pane_id}",
				pane_id: *pane_id,
				on_location_added,
			}
		}

		// Minimized tab bar
		if has_minimized {
			div { class: "picker-tab-bar",
				for (id , label , name) in minimized_panes.iter() {
					{
					    let id = *id;
					    let label = label.clone();
					    let name = name.clone();
					    rsx! {
						div {
							key: "{id}",
							class: "picker-tab",
							onclick: move |_| {
							    picker.restore(id);
							},
							oncontextmenu: move |e: Event<MouseData>| {
							    e.prevent_default();
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
fn PickerPaneView(picker: Store<PickerManager>, pane_id: u64, on_location_added: EventHandler) -> Element {
	let db = use_context::<DbHandle>();

	// Find pane index helper
	let find_pane_idx = move || -> Option<usize> {
		let panes = picker.panes();
		let panes_read = panes.read();
		panes_read.iter().position(|p| p.id == pane_id)
	};

	// Load root dir on mount
	use_effect(move || {
		let Some(idx) = find_pane_idx() else { return };
		let panes = picker.panes();
		let pane_store = panes.index(idx);
		let needs_load = pane_store.columns().is_empty();
		if needs_load {
			let root = pane_store.root_path().cloned();
			let show_hidden = pane_store.show_hidden().cloned();
			spawn(async move {
				let entries = read_dir_sorted(&root, show_hidden).await;
				if let Some(idx) = find_pane_idx() {
					let panes = picker.panes();
					let mut cols = panes.index(idx).columns();
					cols.set(vec![PickerColumn {
						dir_path: root.to_path_buf(),
						entries,
						selected: None,
					}]);
				}
			});
		}
	});

	// Read pane data as a snapshot for rendering
	let pane_data = {
		let panes = picker.panes();
		let panes_read = panes.read();
		panes_read.iter().find(|p| p.id == pane_id).cloned()
	};

	let Some(pane) = pane_data else {
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
						    if let Some(idx) = find_pane_idx() {
						        let panes = picker.panes();
						        let pane_store = panes.index(idx);
						        let new_show_hidden = !pane_store.show_hidden().cloned();
						        pane_store.show_hidden().set(new_show_hidden);
						        pane_store.columns().clear();

						        let root = pane_store.root_path().cloned();
						        spawn(async move {
						            let entries = read_dir_sorted(&root, new_show_hidden).await;
						            if let Some(idx) = find_pane_idx() {
						                let panes = picker.panes();
						                let mut cols = panes.index(idx).columns();
						                cols.set(
						                    vec![
						                        PickerColumn {
						                            dir_path: root.to_path_buf(),
						                            entries,
						                            selected: None,
						                        },
						                    ],
						                );
						            }
						        });
						    }
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
				for (col_idx , col) in columns.iter().enumerate() {
					div { key: "{col_idx}", class: "picker-column",
						for (entry_idx , entry) in col.entries.iter().enumerate() {
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

							    let size_str = if is_dir { String::new() } else { format_size(entry.size) };
							    rsx! {
								div {
									key: "{name}",
									class: "{entry_class}",
									onclick: move |_| {
									    let entry_path = entry_path.clone();
									    async move {
									        let show_hidden = {
									            let Some(idx) = find_pane_idx() else { return };
									            let panes = picker.panes();
									            let pane_store = panes.index(idx);
									            let mut cols = pane_store.columns();
									            let mut cols_write = cols.write();
									            cols_write.truncate(col_idx + 1);
									            if let Some(col) = cols_write.get_mut(col_idx) {
									                col.selected = Some(entry_idx);
									            }
									            drop(cols_write);
									            pane_store.show_hidden().cloned()
									        };
									        if is_dir {
									            let entries = read_dir_sorted(&entry_path, show_hidden).await;
									            if let Some(idx) = find_pane_idx() {
									                let panes = picker.panes();
									                panes
									                    .index(idx)
									                    .columns()
									                    .push(PickerColumn {
									                        dir_path: entry_path.to_path_buf(),
									                        entries,
									                        selected: None,
									                    });
									            }
									        }
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
					    let db = db.clone();
					    move |_| {
					        let sel = sel_path.clone();
					        let cid = container_id.clone();
					        let db = db.clone();
					        spawn(async move {
					            if let Some(path) = sel {
					                let path_str = path.to_string_lossy().to_string();
					                match add_location_from_picker(&db, &cid, &path_str).await {
					                    Ok(()) => {
					                        info!("location added from picker: {}", path_str);
					                        on_location_added.call(());
					                    }
					                    Err(e) => error!("add location failed: {}", e),
					                }
					            }
					        });
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

async fn add_location_from_picker(db: &DbHandle, container_id: &str, path: &str) -> Result<(), String> {
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
