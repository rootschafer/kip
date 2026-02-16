use dioxus::prelude::*;
use std::time::Instant;

// ─── NotificationService Store ────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Progress,
}

#[derive(Store, Debug, Clone, PartialEq)]
pub struct Notification {
    pub id: u32,
    pub message: String,
    pub level: NotificationLevel,
    pub created_at: Instant,
    pub dismissed: bool,
    pub progress: Option<f64>,  // For progress notifications (0.0 to 1.0)
    pub spinner: bool,          // For ongoing operations
}

impl Notification {
    pub fn is_expired(&self) -> bool {
        if self.spinner || matches!(self.level, NotificationLevel::Progress) {
            // Progress/spinner notifications don't expire automatically
            self.dismissed
        } else {
            self.dismissed || Instant::now().duration_since(self.created_at).as_secs() >= 5
        }
    }
}

#[derive(Store, Clone, PartialEq)]
pub struct NotificationService {
    pub notifications: Vec<Notification>,
    pub next_id: u32,
}

impl NotificationService {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            next_id: 0,
        }
    }
}

#[store(pub)]
impl Store<NotificationService> {
    fn add(&mut self, message: String, level: NotificationLevel) {
        let id = self.next_id().cloned();
        self.next_id().set(id + 1);
        self.notifications().push(Notification {
            id,
            message,
            level,
            created_at: Instant::now(),
            dismissed: false,
            progress: None,
            spinner: false,
        });
    }

    fn add_progress(&mut self, message: String, progress: Option<f64>) -> u32 {
        let id = self.next_id().cloned();
        self.next_id().set(id + 1);
        let notification = Notification {
            id,
            message,
            level: NotificationLevel::Progress,
            created_at: Instant::now(),
            dismissed: false,
            progress,
            spinner: progress.is_none(), // Spinner if no progress value provided
        };
        self.notifications().push(notification);
        id
    }

    fn info(&mut self, message: String) {
        self.add(message, NotificationLevel::Info);
    }

    fn warn(&mut self, message: String) {
        self.add(message, NotificationLevel::Warning);
    }

    fn error(&mut self, message: String) {
        self.add(message, NotificationLevel::Error);
    }

    fn progress(&mut self, message: String, progress: Option<f64>) {
        self.add_progress(message, progress);
    }

    fn update_progress(&mut self, id: u32, progress: f64) {
        let notifs = self.notifications();
        let snapshot = notifs.read();
        if let Some(idx) = snapshot.iter().position(|n| n.id == id) {
            drop(snapshot);
            notifs.index(idx).progress().set(Some(progress.clamp(0.0, 1.0)));
        }
    }

    fn start_spinner(&mut self, message: String) -> u32 {
        self.add_progress(message, None)
    }

    fn stop_spinner(&mut self, id: u32) {
        let notifs = self.notifications();
        let snapshot = notifs.read();
        if let Some(idx) = snapshot.iter().position(|n| n.id == id) {
            drop(snapshot);
            notifs.index(idx).dismissed().set(true);
        }
    }

    fn dismiss(&mut self, id: u32) {
        let notifs = self.notifications();
        let snapshot = notifs.read();
        if let Some(idx) = snapshot.iter().position(|n| n.id == id) {
            drop(snapshot);
            notifs.index(idx).dismissed().set(true);
        }
    }

    fn cleanup(&mut self) {
        self.notifications().retain(|n| !n.is_expired());
    }
}

// ─── NotificationLayer Component ──────────────────────────────

#[component]
pub fn NotificationLayer(mut notifs: Store<NotificationService>) -> Element {
    // Auto-cleanup expired notifications every second
    spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            notifs.cleanup();
        }
    });

    let active: Vec<Notification> = notifs
        .notifications()
        .cloned()
        .into_iter()
        .filter(|n| !n.is_expired())
        .collect();

    if active.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "notification-stack",
            for notif in active.iter().rev() {
                {
                    let id = notif.id;
                    let level_class = match notif.level {
                        NotificationLevel::Info => "notif-info",
                        NotificationLevel::Warning => "notif-warning",
                        NotificationLevel::Error => "notif-error",
                        NotificationLevel::Progress => "notif-progress",
                    };
                    let msg = notif.message.clone();
                    let progress = notif.progress;
                    let spinner = notif.spinner;
                    
                    rsx! {
                        div {
                            key: "{id}",
                            class: "notification-toast {level_class}",
                            onclick: move |_| notifs.dismiss(id),
                            
                            if spinner {
                                div { class: "spinner" }
                            }
                            
                            span { class: "notif-message", "{msg}" }
                            
                            if let Some(p) = progress {
                                div { class: "progress-bar-container",
                                    div {
                                        class: "progress-bar-fill",
                                        style: format!("width: {}%;", p * 100.0)
                                    }
                                }
                            }
                            
                            button {
                                class: "notif-close",
                                onclick: move |e: MouseEvent| {
                                    e.stop_propagation();
                                    notifs.dismiss(id);
                                },
                                "\u{2715}"
                            }
                        }
                    }
                }
            }
        }
    }
}
