![Rust](https://img.shields.io/badge/rust-stable-brightgreen)
![Build](https://github.com/pro-grammer-SD/gmtui/actions/workflows/ci.yml/badge.svg)
![License](https://img.shields.io/badge/License-MIT-yellow)
![Platform](https://img.shields.io/badge/platform-linux%20%7C%20macOS%20%7C%20Windows-blue)

GMTUI is a fast, keyboard-driven Gmail client for the terminal. It focuses on cached, low-latency inbox navigation, a composable UI, and async Gmail API calls without blocking the render loop.

## Installation

### Cargo Install

```
cargo install gmtui
```

### Manual Build

```
git clone https://github.com/pro-grammer-SD/gmtui
cd gmtui
cargo build --release
```

## Configuration

`config.toml` (default path: platform config directory + `gmtui/config.toml`)

```
# OAuth (required)
client_id = "YOUR_CLIENT_ID"
client_secret = "YOUR_CLIENT_SECRET"
token_cache_path = "token.json"  # optional, default: config_dir/gmtui/token.json

# UI + cache
theme = "dark"          # dark | light
page_size = 20
cache_ttl_secs = 300

[keybindings]
down = "j"              # also supports Down
up = "k"                # also supports Up
open = "Enter"
compose = "c"
reply = "r"
delete = "d"
search = "/"
refresh = "R"
quit = "q"
next_tab = "Tab"
prev_tab = "Shift+Tab"
load_more = "L"
help = "?"
```

## Keybindings

| Action | Key |
| --- | --- |
| Down | `j` / `Down` |
| Up | `k` / `Up` |
| Open | `Enter` |
| Compose | `c` |
| Reply | `r` |
| Delete | `d` |
| Search | `/` |
| Refresh | `R` |
| Quit | `q` |
| Next Tab | `Tab` |
| Prev Tab | `Shift+Tab` |
| Load More | `L` |
| Help | `?` |

## Search Example

```
$ gmtui
/ rust
  Rust newsletter
  Rust async roundup
<Enter>
```

## Themes

![Screenshot](docs/assets/screenshot.png)

Available themes: `dark`, `light`.

## Feature Checklist

- [x] Channel-based async event loop
- [x] Gmail list/get/send/delete/modify
- [x] Cache with TTL + lazy load
- [x] Search overlay filtering
- [x] Email view with scrolling
- [x] Composer with reply prefill
- [x] Sorting (date/sender/subject)
- [x] Sidebar categories + unread counts
- [x] Theming with dark/light
- [x] Status bar with key hints

## Contributing

See `CONTRIBUTING.md` for development workflow and code style.

License: MIT
