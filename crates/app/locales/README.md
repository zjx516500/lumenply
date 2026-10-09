# Interface translations

The default interface is English. Choose **Language → 简体中文** to switch
immediately; **语言 → English** switches back. The choice is stored as
`language` in the existing `prefs.json`. Older preferences and unsupported
language codes fall back to English without discarding other settings.

`zh-CN.json` maps English source text to Chinese. UI helpers in `theme.rs`
translate shared menus, sliders, buttons and section headings. Other UI
boundaries use `tr(source)`. Section headings also resolve uppercase versions
of catalogue keys. Missing entries fall back to their English source.

Use `tr_args("Undo {action}", &[("action", translated_action)])` for dynamic
messages. Named placeholders may be reordered in translations; argument values
are inserted once and are never parsed as templates. Keep plural suffixes in
English templates; Chinese can omit the suffix placeholder.

Only translate interface text. Keep document text, user layer/preset/action
names, paths, file formats, widget ids, action ids and shortcut keys unchanged.
The frame scopes the selected language to its thread; nested scopes restore
the previous language and independent app contexts cannot leak language state.

To add a language, add its catalogue and extend `i18n::Language` (serialization
code, native label, `ALL`, and `catalogue`). The existing menu and UI helpers
then handle it. Keep the unknown-code fallback variant last for serde.

Tests in `i18n.rs` exercise a real menu click, switching back, saved preferences,
legacy preferences, unchanged document revisions and names, literal argument
insertion, and bundled CJK glyph coverage. Palette tests check Chinese search.
