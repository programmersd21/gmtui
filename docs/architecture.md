# Architecture

## Event Loop

```
+---------------------------+
| crossterm input thread    |
| (poll/read raw events)    |
+-------------+-------------+
              |
              v
   tokio::sync::mpsc::unbounded_channel
              |
              v
      InputEvent enum
              |
              v
   App::handle_event(event)
              |
              v
        AppState mutation
              |
              v
 terminal.draw(|f| ui::render(f, &mut app))
```

## Module Dependency Graph

```
main.rs
  -> app.rs
     -> state.rs
     -> ui.rs
        -> components/*
     -> gmail/client.rs
        -> gmail/models.rs
     -> config.rs
  -> gmail/auth.rs
  -> config.rs
```

## Async Task Communication

- All Gmail API calls run in `tokio::spawn` tasks.
- Each task sends results back to the main loop via `tokio::sync::mpsc::UnboundedSender<InputEvent>`.
- The draw phase is pure: no `.await` or blocking calls occur inside `ui::render`.

## Cache Design and TTL Strategy

- Each mailbox has a `MailboxState` cache with `emails`, `next_page_token`, and `last_fetched`.
- `cache_ttl_secs` from `config.toml` is enforced per mailbox.
- Refresh (`R`) always re-fetches; otherwise fetch is skipped when `last_fetched.elapsed() < ttl`.
- Lazy loading uses `next_page_token` to append additional pages without replacing existing cache.

## Theme System

```
config.toml -> Config::load() -> Theme (dark/light)
        -> components render -> ratatui::Style (no hardcoded colors)
```

All visual styling is derived from `Theme`, which is selected by name and injected into every component.
