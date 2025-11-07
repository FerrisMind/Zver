# Copilot Instructions for Zver

## Architecture Snapshot
- Workspace packages: `crates/zver` (engine library) and `crates/zver-egui` (debug UI) managed via the root `Cargo.toml` workspace.
- `crates/zver/src/lib.rs` exposes the `Zver` struct, wiring DOM, CSS, layout, render, network, JS, and resource loader modules behind `Arc<RwLock<_>>` guards.
- DOM parsing lives in `crates/zver/src/dom.rs`, using `scraper` (Html + ElementRef); selectors like `select_ids` map CSS queries to internal node ids.
- CSS processing (`crates/zver/src/css/`) parses with `cssparser`, caches selectors, and stores `computed_styles` as `HashMap<usize, HashMap<String, String>>` for downstream layout.
- Layout (`crates/zver/src/layout.rs`) builds a Taffy tree per load, applying defaults from `layout/styles` and aggregating inline children; results surface via `LayoutEngine::get_all_layout_results` and `resolved_styles`.
- Rendering (`crates/zver/src/render/`) consumes layout output asynchronously with WGPU; in headless contexts prefer inspecting `layout.collect_render_info` before calling `render.paint`.

## Coding Patterns
- Release locks before awaiting: clone snapshots (`dom.read().await.clone()`) as done in `Zver::load_url` to avoid holding `RwLock` guards across `.await` points.
- Extend CSS support by updating `css/properties.rs` for parsing and `layout/types.rs` for translating into `ComputedStyle`; many map entries assume pixel strings (strip `px`).
- Layout invalidation relies on `LayoutEngine::invalidate`; call it when mutating DOM or styles outside the built-in load pipeline to keep caches consistent.
- Network fetches (`network.rs`) cache by URL; call `clear_cache_for_url` before reloads (see `zver-egui` Reload button) to force fresh fetches.
- `resource_loader.rs` must run `init().await` exactly once before queuing requests via `request_css/image/script`; retrieve finished work with `poll_resources()` inside your async loop.
- Unsafe blocks only mark DOM/Layout structs as `Send/Sync`; avoid introducing new `unsafe` unless you mirror the existing pattern and reason about `Rc` lifetimes.

## Build & Test
- Standard CI gate: `cargo fmt`, `cargo clippy -- -D warnings`, then `cargo test`; run them at the workspace root so both crates participate.
- Targeted CSS progression tests live in `crates/zver/tests/`; e.g. `cargo test --package zver --test css_phase2_tests` or `css_phase5_tests` for specific phases.
- Integration HTML fixtures under `tests/phase*.html` load via `file://` URLs; reuse the pattern from `examples/layout_inspection.rs` when scripting new checks.
- GUI exploration: `cargo run --bin zver-egui` (set `WGPU_BACKEND=dx12` on Windows if WGPU picks the wrong adapter); the app shows DOM/layout stats via `serialize_dom` and `layout.collect_render_info`.
- Verbose CSS debugging uses `RUST_LOG=zver::css=trace` and additional tracing hooks outlined in `docs/BUILD.md`.

## Reference Material
- `docs/ARCHITECTURE.md` maps subsystem responsibilities and data flow; consult before touching cross-cutting logic.
- `docs/BUILD.md` documents platform prerequisites, profiling commands, and WGPU environment knobs.
- `docs/EXAMPLES.md` and `examples/` cover real engine usage patterns, including DOM traversal and layout inspection helpers.
- `tests/README.md` explains each CSS feature phase and expected behaviors; align enhancements with those milestones.
- `CONTRIBUTING.md` captures style rules (Rust 2024, no stray `unsafe`) and preferred review checklist.
