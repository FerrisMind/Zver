# Zver Browser - Quick Start Guide

## Running the Application

```bash
# From repository root
cargo run --bin zver-egui

# With WGPU backend selection (Windows)
$env:WGPU_BACKEND="dx12"
cargo run --bin zver-egui
```

## Using the Browser

### Tab Management

**Create New Tab:**
- Click `➕ New Tab` button in tab bar
- Maximum 5 tabs allowed

**Switch Tabs:**
- Click on tab title in tab bar
- Address bar and DevTools automatically sync

**Close Tab:**
- Click `✖` button on tab
- Cannot close last tab

**Reload Tab:**
- Click `⟳` button in tab bar
- Clears cache before reloading

### Navigation

**Load URL:**
1. Type URL in address bar
2. Press Enter or click `Load`

**Quick Load Test Files:**
1. Click `Test Files` dropdown
2. Select HTML file
3. Loads automatically

**Reload Current Page:**
- Click `⟳` button in address bar
- Or use `⟳` in tab bar

### Developer Tools

**Toggle DevTools:**
- Click `▶` / `▼` button in address bar

**DevTools Tabs:**
- **Elements:** View DOM tree and HTML source
- **Console:** See log messages
- **Network:** (Placeholder - coming soon)
- **Performance:** View layout statistics

**Refresh DevTools:**
- Click `⟳ Refresh` in DevTools panel

**Debug Overlays:**
- Check `Debug Overlays` in Elements tab
- Shows node boundaries and dimensions

## Keyboard Shortcuts

Currently implemented:
- **Enter** in address bar → Load URL

Coming soon:
- **Ctrl+T** → New tab
- **Ctrl+W** → Close tab
- **Ctrl+R** → Reload
- **Ctrl+F** → Find in page

## Supported URLs

**Local Files:**
```
file://tests/test_phase1.html
file://tests/test_phase2.html
file://C:/path/to/file.html
```

**Remote URLs:**
```
http://example.com
https://example.com/page.html
```

## Test Files

The browser auto-scans these directories:
- `tests/`
- `../tests/`
- `../../tests/`

Example test files:
- `test_phase1.html` - Basic HTML/CSS
- `test_phase2.html` - Advanced selectors
- `test_phase3.html` - Pseudo-classes
- `test_phase4.html` - Grid/Flexbox
- `phase*_*.html` - Feature-specific tests

## Troubleshooting

### Application Won't Start

```bash
# Clean build
cargo clean
cargo build --bin zver-egui

# Check for errors
cargo clippy --package zver-egui
```

### Test Files Not Appearing

1. Verify `tests/` directory exists
2. Check files have `.html` extension
3. Try absolute `file://` paths

### Rendering Issues

1. Toggle debug overlays in DevTools
2. Check DOM in Elements tab
3. Verify layout statistics in Performance tab
4. Try different WGPU backend:
   ```bash
   $env:WGPU_BACKEND="vulkan"  # or "dx11", "dx12", "gl"
   ```

### Performance Problems

1. Close unused tabs (max 5)
2. Check DevTools Performance tab
3. Reload page to clear cache
4. Restart application

## Development

### Run Tests

```bash
cargo test --package zver-egui
```

### Check Code Quality

```bash
cargo fmt
cargo clippy --package zver-egui -- -D warnings
```

### View Documentation

```bash
cargo doc --package zver-egui --open
```

## Architecture Overview

```
ZverBrowser
├── TabManager (manages up to 5 tabs)
│   └── Tab[] (each with Arc<Zver> engine)
├── AddressBar (URL input + file dropdown)
├── DevTools (Elements/Console/Network/Performance)
└── RenderView (clean white canvas with scrolling)
```

Each tab has:
- Isolated Zver engine instance
- Independent DOM tree
- Separate layout engine
- Own network cache

## Tips & Tricks

1. **Fast Testing:**
   - Use dropdown for quick file access
   - Keep commonly used files open in tabs

2. **Debugging:**
   - Check Elements tab for DOM structure
   - Use Performance tab for layout metrics
   - Toggle debug overlays to see boundaries

3. **Resource Management:**
   - Close unused tabs
   - 5-tab limit prevents memory issues
   - Each tab has full engine instance

4. **Navigation:**
   - Enter key works in address bar
   - Dropdown auto-selects on click
   - Reload clears cache automatically

## Known Limitations

1. Maximum 5 tabs (by design)
2. No tab history (back/forward)
3. Network panel is placeholder
4. No keyboard shortcuts yet
5. No tab reordering (drag-drop)
6. No session persistence

## Getting Help

1. Check `docs/BROWSER_ARCHITECTURE.md` for detailed design
2. Read `BROWSER_IMPLEMENTATION_SUMMARY.md` for TRIZ analysis
3. View inline documentation with `cargo doc`
4. Check existing test files in `tests/` directory

## Contributing

When adding features:

1. Follow TRIZ principles (see architecture docs)
2. Maintain module boundaries
3. Add tests for new functionality
4. Update documentation
5. Run clippy and fix warnings

## Version Information

- **Current Version:** 1.0.0
- **Rust Edition:** 2024
- **Key Dependencies:**
  - eframe/egui (UI framework)
  - tokio (async runtime)
  - zver (web engine)

## Quick Reference

| Action | Method |
|--------|--------|
| New Tab | Click `➕ New Tab` |
| Close Tab | Click `✖` on tab |
| Switch Tab | Click tab title |
| Load URL | Type URL + Enter |
| Quick Load | Use dropdown |
| Reload | Click `⟳` button |
| DevTools | Click `▶/▼` button |
| Refresh DevTools | Click `⟳ Refresh` |
| Debug View | Check `Debug Overlays` |

---

**For More Information:**
- Architecture: `docs/BROWSER_ARCHITECTURE.md`
- Implementation: `BROWSER_IMPLEMENTATION_SUMMARY.md`
- Source Code: `crates/zver-egui/src/`
