# Display text

`ui.json` maps the Korean source text to `[English, Japanese]`. Korean is the source fallback; language choices use autonyms. The catalog is embedded in the executable and does not need downloading. Windows display language determines System mode; unsupported system languages fall back to English.

Use `i18n::text` for static UI text and `i18n::format` for positional placeholders (`{0}`, `{1}`, ...). Arguments are formatted before insertion and inserted verbatim; never run translation or placeholder replacement over document names, glyph text, credentials, prompts, provider responses, identifiers or serialized document data. Built-in names may be localized when creating a new document, but existing names are never migrated when the UI language changes.

The main UI sets its thread-local language from preferences. A background job captures this language when launched, before it formats progress or errors. Backend error text therefore follows the request's display language without depending on mutable global state. Current-thread Tokio execution retains this context. New threads must explicitly propagate it, as `Studio::spawn` does.

`tests/i18n.rs` checks source-key coverage, nonempty translations, positional placeholders, persisted preferences, unchanged document data, Korean decision commands under each UI language, and translated refinement shortcuts. Language UI QA drives the dropdown with pointer input and restarts the process.
