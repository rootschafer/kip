# UI Testing — what is possible, and what isn't

Written after checking each option against the actual code rather than assuming.
The driving question: **can an agent working in a headless container validate a
UI fix itself — reproduce a bug with a failing test, then make it pass?**

Short answer: partly. Component rendering and state logic, yes. Real pointer
interaction, no.

## What works today: headless SSR rendering

`dioxus-ssr` renders a `VirtualDom` to an HTML string natively. No browser, no
display server, no wasm. It runs anywhere `cargo test` runs, including inside
the Docker image with `--network none`.

```rust
let mut dom = VirtualDom::new(App);
dom.rebuild_in_place();
let html = dioxus_ssr::render(&dom);
```

Working examples in `frontend/tests/ui_render_tests.rs`.

This covers more than it first appears, because Kip's graph puts its layout
into the markup — node geometry lands in inline styles, selection state lands in
classes. So it catches:

- markup structure and conditional branches in `rsx!`
- computed geometry (a layout regression changes the emitted `style`)
- selection / pinned / visible state affecting output
- text and label rendering
- props flowing correctly through the component tree

Most of the graph bugs in this project's history were layout math and state
bookkeeping. Those are all reachable this way.

**Setup cost is low**: only the four top-level containers (`graph.rs`,
`file_picker.rs`, `review_queue.rs`) call `use_context::<DbHandle>()`. Every
leaf component — nodes, edges, context menu, notifications — is prop-driven and
renders with no database at all. For the containers, `daemon` already builds
SurrealDB with `kv-mem`, so a test can spin up an in-memory instance.

## What does not work: pointer interaction

SSR renders one frame. It does not dispatch events, so drag gestures, hover,
click sequences, and anything depending on a real event loop are out of reach.
Browser-computed layout (flexbox resolution, actual hit-testing) is also out —
you are asserting on the markup Kip *emits*, not on what a renderer *does* with
it.

## Why Playwright is not available

Playwright drives a browser, so it needs a web build. There isn't one, and
getting one is not a small change.

Verified directly:

```sh
cargo check -p frontend --target wasm32-unknown-unknown --no-default-features --features web
```

fails immediately with `This wasm target is unsupported by mio. If using Tokio,
disable the net feature.` — hundreds of errors before reaching any Kip code.

The cause is structural, not a missing flag:

```
frontend  →  daemon, kip-core  →  surrealdb (kv-surrealkv / kv-mem)  →  tokio net+fs  →  mio  →  ✗ wasm
```

`frontend` also depends on `surrealdb` directly and on `tokio` with
`features = ["full"]`. The `web` feature in `frontend/Cargo.toml` is vestigial:
turning it on does not remove any of that, because those dependencies are not
optional.

Making a web build work would require splitting the UI from the data layer so
the wasm bundle links neither SurrealDB nor native tokio, and talking to a
native backend over HTTP or a websocket instead. That is Dioxus fullstack in all
but name — which `CLAUDE.md` lists under "Decisions That Are Final" as
explicitly rejected ("**No Dioxus fullstack**. Desktop only.").

So this is a real architectural fork, not a chore. It should not be started
without deciding to reverse that decision.

## Other options considered

| Option | Verdict |
|---|---|
| `dx serve --platform web` | Same wasm wall as above. |
| Xvfb + screenshot the desktop app | Runs the real WebKit window headlessly, but assertions become image diffs — brittle, and an agent cannot easily turn "looks wrong" into a failing test. Also drags a large X stack into the image. |
| `dioxus-desktop` in-process with JS eval | Still needs a real window and WebKit; no display in the container. |
| Unit tests on layout/graph logic | **Cheapest and highest value.** The force-directed layout, path containment, node geometry and selection math in `kip-core/src/graph_types.rs` and `daemon/src/graph_store.rs` are pure functions. Test them directly. |

## Recommendation

For an agent expected to fix UI bugs with failing-test-then-green:

1. **Push logic out of components.** Anything computing positions, containment,
   selection or visibility should be a pure function in `kip-core` or
   `graph_store`, unit-tested directly. This is where the bugs are and it needs
   no rendering at all.
2. **Use SSR tests for the rendering contract** — that state actually reaches
   the markup.
3. **Accept that gesture-level behaviour needs a human**, or reopen the
   fullstack decision deliberately if browser automation becomes a hard
   requirement.

Framing a UI task for a container-bound agent should say which of these three
buckets the bug falls into. A "the drag feels wrong" bug is not currently
verifiable there; "the node renders at the wrong x when its parent is
collapsed" very much is.
