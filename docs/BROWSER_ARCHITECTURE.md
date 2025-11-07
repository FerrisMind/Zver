# Zver Browser Architecture

## Overview

The Zver Browser is a transformation of the monolithic egui demo into a full-featured browser-like interface using TRIZ (Theory of Inventive Problem Solving) principles. This document describes the modular architecture and design patterns implemented.

## TRIZ Principles Applied

### 1. Drobleniye (Segmentation)
**Problem:** Monolithic application with scattered functionality  
**Solution:** Break down into independent, focused modules

The application is divided into four core modules:
- **Tab** - Manages individual browsing contexts with isolated engine instances
- **AddressBar** - Handles navigation and URL input
- **DevTools** - Provides diagnostic and debugging capabilities
- **RenderView** - Displays rendered page content

### 2. Dinamichnost (Dynamicity)
**Problem:** Static interface with fixed capabilities  
**Solution:** Dynamic component behavior adapting to context

- Tabs can be added/removed up to 5 instances (resource constraint)
- DevTools panel shows/hides on demand
- HTML file scanner auto-discovers test files
- UI adapts to tab state (Loading/Loaded/Error)

### 3. Vynesenie (Taking Out)
**Problem:** Mixing diagnostic tools with primary interface  
**Solution:** Extract DevTools into independent, toggleable panel

DevTools are separated into their own panel with:
- Independent state management
- Synchronization with active tab
- Multiple specialized sub-panels (Elements, Console, Network, Performance)

### 4. Obedinenie (Merging)
**Problem:** Duplicate rendering logic across components  
**Solution:** Unified interface with shared abstractions

- All tabs share the same `Arc<Zver>` interface
- RenderView provides single rendering pipeline
- Common `TabManager` coordinates all tab operations

## Module Structure

```
crates/zver-egui/src/
├── main.rs                    # ZverBrowser main application
├── egui_integration.rs        # Rendering utilities
└── browser/
    ├── mod.rs                 # Module exports
    ├── tab.rs                 # Tab + TabManager
    ├── address_bar.rs         # AddressBar component
    ├── devtools.rs            # DevTools panel
    └── render_view.rs         # RenderView component
```

## Component Architecture

### TabManager
**Responsibility:** Manage multiple browsing tabs with isolated engine instances

**Key Features:**
- Maximum 5 tabs (resource management)
- Each tab has isolated `Arc<Zver>` engine
- Active tab tracking
- Tab creation/deletion with state preservation

**API:**
```rust
pub fn new(runtime: Arc<Runtime>) -> Self
pub fn add_tab(&mut self) -> bool
pub fn close_tab(&mut self, index: usize) -> bool
pub fn get_active_tab(&self) -> Option<&Tab>
pub fn load_url_in_active_tab(&mut self, url: String)
pub fn reload_active_tab(&mut self)
```

### Tab
**Responsibility:** Represent a single browsing context

**Fields:**
- `id: usize` - Unique identifier
- `title: String` - Display title (extracted from URL)
- `url: String` - Current URL
- `engine: Arc<Zver>` - Isolated engine instance
- `status: TabStatus` - Current state (Idle/Loading/Loaded/Error)

**State Machine:**
```
Idle → Loading → Loaded
            ↓
          Error
```

### AddressBar
**Responsibility:** Handle URL input and navigation controls

**Key Features:**
- Text input with Enter key support
- Auto-scan `tests/` directory for HTML files
- Dropdown selector for test files
- DevTools toggle button (▶/▼)
- Load/Reload buttons

**Auto-Discovery:**
Scans relative paths: `tests/`, `../tests/`, `../../tests/`

### DevTools
**Responsibility:** Provide diagnostic and debugging tools

**Panels:**
1. **Elements** - DOM tree with HTML serialization
2. **Console** - Log messages and JS output
3. **Network** - Request monitoring (placeholder)
4. **Performance** - Layout metrics and timing

**Synchronization:**
DevTools update when:
- Tab switches
- URL loads
- Manual refresh

### RenderView
**Responsibility:** Display rendered page content

**Features:**
- Clean white background
- Scrollable viewport
- Content size calculation from `LayoutResult`
- Reuses existing `render_clean_layout_from_results()`

**Canvas Sizing:**
```rust
canvas_width = max(content_width + 20.0, 800.0)
canvas_height = max(content_height + 20.0, 600.0)
```

## UI Layout

The application uses egui's panel system:

```
┌─────────────────────────────────────┐
│ TopBottomPanel::top("tabs_panel")  │ ← Tab bar
├─────────────────────────────────────┤
│ TopBottomPanel::top("address_bar")  │ ← Address bar
├─────────────────────────────────────┤
│                                     │
│ CentralPanel                        │ ← Render view
│                                     │
├─────────────────────────────────────┤
│ TopBottomPanel::bottom("devtools")  │ ← DevTools (if open)
└─────────────────────────────────────┘
```

## State Management

### Async Operations
All blocking async operations use `runtime.block_on()`:
- URL loading
- Cache clearing
- DOM/Layout reading

### Borrow Patterns
To avoid borrowing conflicts:
1. Clone `Arc<Runtime>` before mutable borrows
2. Collect tab info before iterating
3. Drop locks before rendering

Example:
```rust
let runtime = Arc::clone(&self.runtime);
if let Some(tab) = self.get_active_tab_mut() {
    tab.load_url(url, &runtime);
}
```

## Resource Management

### Tab Limits
Maximum 5 tabs enforced by `TabManager::MAX_TABS` constant.

**Rationale:**
- Prevent memory exhaustion (each tab has full engine)
- Maintain UI responsiveness
- Simplify testing

### Engine Isolation
Each tab has `Arc<Zver>` instance:
- Independent DOM trees
- Separate layout engines
- Isolated network caches

## Testing Strategy

### Manual Testing
1. **Tab Management:**
   - Create tabs (up to 5)
   - Switch between tabs
   - Close tabs (keep last one)

2. **Navigation:**
   - Enter URLs manually
   - Select from dropdown
   - Use Enter key
   - Reload functionality

3. **DevTools:**
   - Toggle panel visibility
   - Switch between tabs (Elements/Console/Network/Performance)
   - Verify DOM synchronization
   - Check console logs

4. **Rendering:**
   - Load test HTML files
   - Verify clean white background
   - Check scrolling behavior
   - Confirm responsive sizing

### Automated Testing
```bash
cargo clippy --package zver-egui -- -D warnings
cargo test --package zver-egui
```

## Known Limitations

1. **Network Panel:** Placeholder implementation
2. **Performance Metrics:** Basic statistics only
3. **Tab Limit:** Hard-coded to 5
4. **No Tab Persistence:** State lost on close

## Future Enhancements

### Phase 1 (Immediate)
- [ ] Tab history (back/forward)
- [ ] Keyboard shortcuts (Ctrl+T, Ctrl+W)
- [ ] Tab reordering (drag-and-drop)

### Phase 2 (Near-term)
- [ ] Bookmarks system
- [ ] Download manager
- [ ] Search bar (Ctrl+F)
- [ ] Network request details

### Phase 3 (Long-term)
- [ ] Multi-window support
- [ ] Session persistence
- [ ] Extensions API
- [ ] Custom DevTools themes

## Coding Standards

### Rust Edition
`rust-edition = "2024"`

### Style Guidelines
- `snake_case` for functions and variables
- `PascalCase` for types and enums
- `SCREAMING_SNAKE_CASE` for constants
- `///` doc comments for public APIs

### Error Handling
- Use `Result<T, E>` for fallible operations
- Log errors to DevTools console
- Update tab status on failures

### Async Patterns
- Block on async operations in UI thread
- Clone `Arc` references before mutable borrows
- Release locks before `.await` points

## Dependencies

Key crates:
- `eframe` / `egui` - UI framework
- `tokio` - Async runtime
- `zver` - Web engine (local crate)

## Maintenance

### Adding New DevTools Tabs
1. Add variant to `DevToolsTab` enum
2. Implement `render_*_tab()` method
3. Update `DevToolsTab::all()` array
4. Add to tab selector UI

### Extending Tab Functionality
1. Add fields to `Tab` struct
2. Implement logic in `Tab` impl
3. Expose via `TabManager` if needed
4. Update UI in `render_tab_bar()`

## Troubleshooting

### Build Errors
```bash
cargo clean
cargo build --package zver-egui
```

### Runtime Issues
- Check `RUST_LOG=debug cargo run --bin zver-egui`
- Verify `tests/` directory exists
- Confirm WGPU backend: `set WGPU_BACKEND=dx12`

### UI Glitches
- Ensure single-threaded async operations
- Verify no locks held during rendering
- Check for borrow conflicts in clippy

## References

- [TRIZ in Software Development](http://2017.secr.ru/program/submitted-presentations/applications-of-triz-methods-in-sw-development)
- [egui Documentation](https://docs.rs/egui/latest/egui/)
- [Zver Architecture](../../../docs/ARCHITECTURE.md)

---

**Last Updated:** 2025-11-08  
**Version:** 1.0.0  
**Author:** TRIZ-Based Architecture Team
