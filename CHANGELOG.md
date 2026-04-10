## What's New in v0.6.0

### New Features

- **SVG optimization** — SVG files are now supported via vexy-vsvg (a native Rust port of SVGO), removing metadata, minifying paths, and collapsing groups for smaller file sizes
- **Internationalization** — the app is now available in 6 languages: English, Dutch, German, French, Spanish, and Russian
- **Toast notifications** — non-intrusive feedback when unsupported files are skipped, with auto-dismiss animation

### Improvements

- **Structured logging** — configurable log levels via `RUST_LOG`
- **UI accessibility** — consistent focus indicators on all interactive elements
- **Typography consistency** — standardized font weights and spacing across panels and menus
- **Dropdown styling** — added chevron icon to the language selector
- **Skipped-files messaging** — shows one filename with a count of others instead of listing all
- **Progress bar responsiveness** — visual now closely tracks the percentage text
- **Unified progress tracking** — progress bar now accurately reflects overall job completion
- Updated JS and Rust dependencies

### Fixes

- Fixed toast notification only appearing once for repeated actions
- Fixed progress events being dropped during fade-in animation
- Fixed processing timer not starting for fast operations