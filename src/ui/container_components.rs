use dioxus::prelude::*;

use crate::ui::graph_types::ContainerView;

// ─── MachineChip ─────────────────────────────────────────────
// Toolbar button representing a machine or drive.
// Clicking opens the file picker for that target.

#[component]
pub fn MachineChip(container: ContainerView, on_click: EventHandler<ContainerView>) -> Element {
	let name = container.name.clone();
	let color = container.color.clone();
	let connected = container.connected;
	let kind_label = if connected {
		container.kind.as_str()
	} else {
		"offline"
	};
	let opacity = if connected { "1" } else { "0.5" };

	rsx! {
		button {
			class: "machine-chip",
			style: "opacity: {opacity};",
			disabled: !connected,
			onclick: move |_| on_click.call(container.clone()),
			div { class: "chip-dot", style: "background: {color};" }
			span { class: "chip-name", "{name}" }
			span { class: "chip-kind", "{kind_label}" }
		}
	}
}
