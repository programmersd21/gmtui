# Contributing to gmtui

Thanks for taking the time to contribute. I maintain `gmtui` for daily use, so changes need to be fast, reliable, and easy to reason about. If that matches your goals, welcome.

## Principles

- Keep the UI fast and keyboard-first.
- Minimize Gmail API calls and respect rate limits.
- Prefer explicit errors over silent recovery.
- Avoid adding new dependencies without prior discussion.

## Code Style

- Keep modules focused and cohesive.
- Use `Result<T, GmtuiError>` for fallible logic.
- Avoid `unwrap()` and `expect()` outside `main.rs`.
- Add tests for logic changes when practical.

## Commit Messages

Use Conventional Commits:

- `feat: add threaded view`
- `fix: handle empty payload body`
- `docs: clarify OAuth setup`

## Running Tests

```bash
cargo test --all
```

Formatting and linting:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
```

## Pull Requests

- Open an issue for UX or API changes first.
- Keep PRs small and focused.
- Describe behavior changes clearly.
- Include tests or explain why they’re not feasible.

## Issue Triage

- Bugs: include steps to reproduce and expected vs actual results.
- Features: explain the user story and tradeoffs.
- Performance: include measurements where possible.

## License

By contributing, you agree that your work is released under the MIT License.

Name: Soumalya Das  
Email: geniussantu1983@gmail.com  
GitHub: https://github.com/programmersd21  
Repo: https://github.com/programmersd21/gmtui
