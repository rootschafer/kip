use std::path::{Path, PathBuf};

use dioxus::prelude::*;
// use dioxus::signals::{Store, Writable, Readable};
use dioxus::signals::*;
use tracing::{error, info, warn};

use crate::db::DbHandle;

// ─── Pane ID generator ──────────────────────────────────────

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn next_pane_id() -> u64 {
    NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

// ─── Data types ─────────────────────────────────────────────

// #[derive(Debug, Clone, PartialEq)]
#[derive(Store, Debug, Clone, PartialEq)]
pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

// #[derive(Debug, Clone, PartialEq)]
#[derive(Store, Debug, Clone, PartialEq)]
pub struct PickerColumn {
    pub dir_path: PathBuf,
    pub entries: Vec<FsEntry>,
    pub selected: Option<usize>,
}

// #[derive(Debug, Clone, PartialEq)]
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

// type MappedPickerPaneDataStore<Lens> = Store<String, MappedMutSignal<String, Lens, fn(&PickerPaneData) -> Iterator<Item = PickerPaneData>>>;
type MappedPickerPaneDataStore<Lens> = Store<String, MappedMutSignal<String, Lens, fn(&PickerPaneData)>>;

#[store]
impl<Lens> Store<PickerPaneData> {
    /// Short label for a path (last 1-2 components).
    fn root_path_label(&self) -> String {
        // let parts: Vec<&str> = self().root_path
        //     .components()
        //     .filter_map(|c| c.as_os_str().to_str())
        //     .collect();

        // if parts.len() <= 2 {
        if self().root_path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect::<Vec<&str>>().len() <= 2 {
            // path.to_string_lossy().to_string()
            self().root_path.to_string_lossy().to_string()
        } else {
            // parts[parts.len() - 2..].join("/")
            self().root_path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect::<Vec<&str>>()[self().root_path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect::<Vec<&str>>().len() - 2..].join("/")
        }

        // match self().root_path
        //     .components()
        //     .filter_map(|c| c.as_os_str().to_str())
        //     .collect()
        // {
        //
        // }
    }


    // // This method does not require any bounds on the lens since it takes `self`
    // fn into_parts(self) -> (MappedPickerPaneDataStore<Lens>, MappedPickerPaneDataStore<Lens>, MappedPickerPaneDataStore<Lens>, MappedPickerPaneDataStore<Lens>, MappedPickerPaneDataStore<Lens>, MappedPickerPaneDataStore<Lens>, MappedPickerPaneDataStore<Lens>) where Self: Copy {
    //     (self.id(), self.container_id(), self.container_name(), self.root_path(), self.columns(), self.minimized(), self.show_hidden())
    // }
}

// ─── Shared state (provided as context) ─────────────────────

#[derive(Store, Clone, PartialEq)]
// pub struct PickerManager(pub Vec<PickerPaneData>);
pub struct PickerManager {
    // panes: Store<Vec<PickerPaneData>>,
    pub panes: Vec<PickerPaneData>,
}

// type MappedUserDataStore<Lens> = Store<String, MappedMutSignal<String, Lens, fn(&UserData) -> &String, fn(&mut UserData) -> &mut String>>;
// type MappedPickerMangerStore<Lens> = Store<String, MappedMutSignal<String, Lens, fn(&PickerManager) -> Iterator<Item = PickerPaneData>, fn(&PickerManager, String, String, PathBuf), fn(&mut PickerManger, u64), fn(&mut PickerManger, u64), fn(&mut PickerManger, u64), fn(&PickerManager) -> bool>>;
type MappedPickerMangerStore<Lens> = Store<String, MappedMutSignal<String, Lens, fn(&PickerManager) -> bool>>;

#[store]
impl<Lens> Store<PickerManager, Lens> {
    // fn iter(&self) -> impl Iterator<Item = PickerPaneData> {
    //     self().0.iter()
    // }

    //
    // // This method does not require any bounds on the lens since it takes `self`
    // fn into_parts(self) -> (MappedUserDataStore<Lens>, MappedUserDataStore<Lens>) where Self: Copy {
    //     (self.email(), self.name())
    // }


    fn open(&mut self, container_id: String, container_name: String, root: PathBuf) {
        // let mut panes = self.write();
        // If pane exists for this container, restore it
        // if let Some(pane) = panes.iter_mut().find(|p| p.container_id == container_id) {
        if let Some(mut pane) = self.panes().iter_mut().find(|p| p.container_id == container_id) {
            pane.minimized = false;
            return;
        }
        info!("opening picker for {} at {:?}", container_name, root);
        // panes.push(PickerPaneData {
        (self.panes())().push(PickerPaneData {
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
        // self.write().retain(|p| p.id != id);
        // self.panes().retain(|p| p.id != id);
        (self.panes())().retain(|p| p.id != id);
    }

    fn minimize(&mut self, id: u64) {
        // if let Some(p) = self.write().iter_mut().find(|p| p.id == id) {
        if let Some(mut p) = (self.panes())().iter_mut().find(|p| p.id == id) {
            p.minimized = true;
        }
    }

    fn restore(&mut self, id: u64) {
        // if let Some(p) = self.write().iter_mut().find(|p| p.id == id) {
        if let Some(p) = (self.panes())().iter_mut().find(|p| p.id == id) {
            p.minimized = false;
        }
    }

    fn has_any(&self) -> bool {
        // !self.read().is_empty()
        // !self().panes().is_empty()
        !(self.panes())().is_empty()
    }

}


impl PickerManager {
    pub fn new() -> Self {
        // Self(Vec::new())
        // Self(Store::new(Vec::new()))
        Self {
            panes: vec![],
        }
    }

    // pub fn open(&mut self, container_id: String, container_name: String, root: PathBuf) {
    //     let mut panes = self.write();
    //     // If pane exists for this container, restore it
    //     if let Some(pane) = panes.iter_mut().find(|p| p.container_id == container_id) {
    //         pane.minimized = false;
    //         return;
    //     }
    //     info!("opening picker for {} at {:?}", container_name, root);
    //     panes.push(PickerPaneData {
    //         id: next_pane_id(),
    //         container_id,
    //         container_name,
    //         root_path: root,
    //         columns: vec![],
    //         minimized: false,
    //         show_hidden: false,
    //     });
    // }
    //
    // pub fn close(&mut self, id: u64) {
    //     self.write().retain(|p| p.id != id);
    // }
    //
    // pub fn minimize(&mut self, id: u64) {
    //     if let Some(p) = self.write().iter_mut().find(|p| p.id == id) {
    //         p.minimized = true;
    //     }
    // }
    //
    // pub fn restore(&mut self, id: u64) {
    //     if let Some(p) = self.write().iter_mut().find(|p| p.id == id) {
    //         p.minimized = false;
    //     }
    // }
    //
    // pub fn has_any(&self) -> bool {
    //     !self.read().is_empty()
    // }
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
    // let panes = picker.read().0.clone();

    // if panes.is_empty() {
    // if picker.panes().is_empty() {
    if picker.panes().is_empty() {
        return rsx! {};
    }

    // let has_minimized = panes.iter().any(|p| p.minimized);

    rsx! {
		// Open panes
		// for pane in panes.iter().filter(|p| !p.minimized) {
		for pane in picker.panes().iter().filter(|p| !p.minimized()) {
			PickerPaneView {
				picker,
				key: "{pane.id}",
				pane_id: pane.id(),
				on_location_added,
			}
		}

		// Minimized tab bar
		if picker.panes().iter().any(|p| p.minimized) {
			div { class: "picker-tab-bar",
				for pane in picker.panes().iter().filter(|p| p.minimized()) {
					div {
						key: "{pane.root_path_label}",
						class: "picker-tab",
						onclick: move |_| {
						    picker.restore(pane.id());
						},
						oncontextmenu: move |e: Event<MouseData>| {
						    e.prevent_default();
						    picker.close(pane.id());
						},
						span { class: "picker-tab-name", "{pane.container_name}" }
						span { class: "picker-tab-path", "{pane.root_path_label}" }
					}
				}
			}
		}
	}
}

// ─── Single pane ────────────────────────────────────────────

#[component]
// fn PickerPaneView(pane_id: u64, on_location_added: EventHandler) -> Element {
fn PickerPaneView(picker: Store<PickerManager>, pane_id: u64, on_location_added: EventHandler) -> Element {
    let db = use_context::<DbHandle>();

    // Load root dir on mount
    use_effect(move || {
        let needs_load = {
            // let panes = picker.read().panes();
            picker.panes()
                .iter()
                .find(|p| p.id() == pane_id)
                .map(|p| p.columns().is_empty())
                .unwrap_or(false)
        };
        if needs_load {
            let root = {
                // let panes = picker.read().panes.read();
                // panes
                picker.panes()
                    .iter()
                    .find(|p| p.id() == pane_id)
                    .map(|p| (p.root_path().clone(), p.show_hidden()))
            };
            if let Some((root, show_hidden)) = root {
                spawn(async move {
                    let entries = read_dir_sorted(&root, show_hidden).await;
                    let mut picker_binding = picker.write();
                    if let Some(p) = picker_binding.panes().iter_mut().find(|p| p.id == pane_id) {
                        p.columns = vec![PickerColumn {
                            dir_path: root.to_path_buf(),
                            entries,
                            selected: None,
                        }];
                    }
                });
            }
        }
    });

    let maybe_pane_memo = use_memo(move || picker.read().panes.read().iter().find(|p| p.id == pane_id).cloned());

    // let container_name = pane.container_name.clone();
    // let container_id = pane.container_id.clone();
    // let columns = pane.columns.clone();
    // let show_hidden = pane.show_hidden;

    // Compute selected path for the bottom bar
    // let sel_path = selected_path(&columns);
    let sel_path = use_memo(move || {
        match maybe_pane_memo.read().as_ref() {
            Some(pane) => selected_path(&pane.columns),
            None => None,
        }
    });
    let sel_display = sel_path()
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let has_selection = sel_path().is_some();

    // // Breadcrumb: show the path of the last column
    // let breadcrumb = columns
    //     .last()
    //     .map(|c| c.dir_path.to_string_lossy().to_string())
    //     .unwrap_or_default();

    let breadcrumb = use_memo(move || {
        match maybe_pane_memo.read().as_ref() {
            Some(pane) => pane.columns.last().map(|c| c.dir_path.to_string_lossy().to_string()).unwrap_or_default(),
            None => String::new(), // Return empty string instead of None
        }
    });

    match maybe_pane_memo.read().as_ref() {
        Some(pane) => rsx! {
			div {
				class: "picker-pane",
				onclick: move |e: MouseEvent| e.stop_propagation(),

				// Title bar
				div { class: "picker-title-bar",
					span { class: "picker-title", "{pane.container_name}" }

					div { class: "picker-title-actions",
						// Toggle hidden files
						button {
							class: if pane.show_hidden { "picker-btn-toggle active" } else { "picker-btn-toggle" },
							title: "Toggle hidden files",
							onclick: move |_| {
							    // 1. Toggle the flag and clear columns
							    {
							        let mut panes = picker.write().panes.write();
							        if let Some(p) = panes.iter_mut().find(|p| p.id == pane_id) {
							            p.show_hidden = !p.show_hidden;
							            p.columns.clear();
							        }
							    }

							    // 2. Reload root directory with new show_hidden setting
							    spawn(async move {
							        let (root, show_hidden) = {
							            // let panes = picker().0.read();
							            let panes = picker().panes.read();
							            panes
							                .iter()
							                .find(|p| p.id == pane_id)
							                .map(|p| (p.root_path.clone(), p.show_hidden))
							                .unwrap_or_else(|| (std::path::PathBuf::from("/"), false))
							        };
							        let entries = read_dir_sorted(&root, show_hidden).await;
							        let mut panes = picker.write().panes.write();
							        if let Some(p) = panes.iter_mut().find(|p| p.id == pane_id) {
							            p.columns = vec![
							                PickerColumn {
							                    dir_path: root.to_path_buf(),
							                    entries,
							                    selected: None,
							                },
							            ];
							        }
							    });
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
							// onclick: move |_| picker().close(pane_id),
							onclick: move |_| picker.close(pane_id),
							"\u{00d7}" // multiplication sign (×)
						}
					}
				}

				// Breadcrumb
				div { class: "picker-breadcrumb", "{breadcrumb}" }

				// Column view
				div { class: "picker-columns",
					// for (col_idx , col) in columns.iter().enumerate() {
					for (col_idx , col) in pane.columns.iter().enumerate() {
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
										            let Some(p) = picker
										                .write()
										                .panes
										                .write()
										                .iter_mut()
										                .find(|p| p.id == pane_id) else { return };
										            p.columns.truncate(col_idx + 1);
										            if let Some(col) = p.columns.get_mut(col_idx) {
										                col.selected = Some(entry_idx);
										            }
										            p.show_hidden
										        };
										        if is_dir {
										            let entries = read_dir_sorted(&entry_path, show_hidden).await;
										            let mut panes = picker.write().panes.write();
										            if let Some(p) = panes.iter_mut().find(|p| p.id == pane_id) {
										                p.columns
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
						onclick: move |_| {
						    async move {
						        if let Some(path) = sel_path() {
						            let path_str = path.to_string_lossy().to_string();
						            match add_location_from_picker(&db, &pane.container_id, &path_str).await
						            {
						                Ok(()) => {
						                    info!("location added from picker: {}", path_str);
						                    on_location_added.call(());
						                }
						                Err(e) => error!("add location failed: {}", e),
						            }
						        }
						    }
						},
						"Add to workspace"
					}
				}
			}
		},
        None => { rsx! {} },
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
