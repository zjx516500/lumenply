# Chinese interface font

`NotoSansSC.ttf` is Noto Sans SC from the Google Fonts repository:
https://github.com/google/fonts/tree/main/ofl/notosanssc

Downloaded from `NotoSansSC[wght].ttf` on 2026-10-09 and instantiated at
weight 400 (Regular) with fontTools. The source defaults to weight 100;
using a static Regular instance keeps Chinese text readable in egui.
The full glyph set is
bundled so the interface works offline on macOS, Windows and Linux without
requiring a system Chinese font. Plex remains the primary Latin font;
Noto Sans SC is a fallback in proportional, monospace and semibold families.

Licensed under the SIL Open Font License 1.1; see `OFL-NotoSansSC.txt`.
