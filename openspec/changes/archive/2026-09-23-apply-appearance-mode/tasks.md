# Tasks

## 1. Resolution, as a pure function

- [x] 1.1 Add `crates/knot/src/appearance.rs` with `resolve(mode, system: WindowAppearance) -> ThemeMode` — `ThemeMode` because that is what `Theme::change` consumes, and it collapses the platform's four appearances to the two the theme has: `Light`/`Dark` return their own mode, `System`/`Auto` convert `system`; verify the `match` on `AppearanceMode` is exhaustive with no `_` arm, so a new variant fails to compile here
- [x] 1.2 Add tests covering all four variants against both a light and a dark OS appearance, including the vibrant variants the platform can report; verify they run without a window

## 2. The preference the app reads

- [x] 2.1 Add an `AppearancePreference` global holding the current `AppearanceMode`, following the `AwaitingInput`/`Activation` pattern in `app_support.rs`; verify it is installed at bootstrap from the loaded settings, before the first window opens
- [x] 2.2 Add `apply(window, cx)` to the appearance module, resolving through the global and calling `Theme::change` followed by `apply_system_palette`; verify the palette re-application is not dropped — `Theme::change` reloads the light or dark config wholesale and discards the overlay

## 3. Startup and OS flips

- [x] 3.1 Replace `Theme::change(cx.window_appearance(), None, cx)` in `app_bootstrap.rs` with the resolved call; verify launching with a stored `dark` on a light Mac opens dark
- [x] 3.2 Change `observe_system_appearance` in `app_support.rs` to resolve through the global rather than pass `cx.window_appearance()` to `Theme::change` directly; verify a light/dark flip repaints under `System` and is ignored under `Light` and `Dark`
- [x] 3.3 Verify every window registering the observer still shares one app-wide appearance — the observer is per-window and the preference is global, so no window may end up painting a different appearance from another

## 4. The picker applies

- [x] 4.1 Change the Appearance picker's handler in `settings_window/panes/general.rs` to update the global and re-apply alongside its existing `persist()`; verify the setting is still saved as `"dark"` and now also repaints
- [x] 4.2 Repaint every open window, not just the settings window; verify a workspace window open behind the settings window repaints without being focused first
- [x] 4.3 Rewrite `settings.general.appearance_hint` in `crates/knot-core/locales/en.yml` to describe what the picker does in this app; verify the l10n key still resolves and the test asserts the key, not the English copy — and `touch` `knot-core` so the locale is not read from a stale build artifact

## 5. Gate

- [x] 5.1 Run `make` end to end — `fmt-check`, `size-check`, `lint`, `test`, `build`; verify no crate-wide `allow` was added and no file crossed 700 lines
- [ ] 5.2 Verify by hand in a debug build: pick each of the four modes, flip the OS appearance under each, and relaunch under an explicit choice
