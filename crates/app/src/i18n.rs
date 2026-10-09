//! UI-only localization. English source text is the catalogue key; action
//! ids, document data and file formats never depend on the selected language.
//! Each frame scopes its language to the UI thread (also isolates test contexts).
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum Language {
    #[serde(rename = "zh-CN", alias = "Chinese")]
    Chinese,
    #[default]
    #[serde(rename = "en", alias = "English", other)]
    English,
}

impl Language {
    pub(crate) const ALL: [Self; 2] = [Self::English, Self::Chinese];

    pub(crate) fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Chinese => "简体中文",
        }
    }

    fn catalogue(self) -> Option<&'static BTreeMap<String, String>> {
        static CHINESE: OnceLock<BTreeMap<String, String>> = OnceLock::new();
        match self {
            Self::English => None,
            Self::Chinese => Some(CHINESE.get_or_init(|| {
                let mut catalogue: BTreeMap<String, String> =
                    serde_json::from_str(include_str!("../locales/zh-CN.json"))
                        .expect("the bundled Chinese catalogue is valid JSON");
                // Section headings derive uppercase variants of enum names.
                // Explicit catalogue entries always take precedence.
                let uppercase: Vec<_> = catalogue
                    .iter()
                    .map(|(key, value)| (key.to_ascii_uppercase(), value.clone()))
                    .collect();
                for (key, value) in uppercase {
                    catalogue.entry(key).or_insert(value);
                }
                catalogue
            })),
        }
    }

    pub(crate) fn enter(self) -> LanguageScope {
        LanguageScope(CURRENT.with(|c| c.replace(self)))
    }
}

thread_local! {
    static CURRENT: Cell<Language> = const { Cell::new(Language::English) };
}

pub(crate) struct LanguageScope(Language);
impl Drop for LanguageScope {
    fn drop(&mut self) {
        set_current(self.0);
    }
}

pub(crate) fn set_current(language: Language) {
    CURRENT.with(|c| c.set(language));
}

pub(crate) fn tr(source: impl Into<String>) -> String {
    let source = source.into();
    CURRENT.with(|c| {
        c.get()
            .catalogue()
            .and_then(|catalogue| catalogue.get(&source))
            .cloned()
            .unwrap_or(source)
    })
}

/// Named placeholders allow translators to reorder arguments. Values are
/// inserted once, so braces in a filename or user text are kept literally.
pub(crate) fn tr_args(source: &str, args: &[(&str, String)]) -> String {
    let template = tr(source);
    let mut result = String::with_capacity(template.len());
    let mut rest = template.as_str();
    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find('}') else { break };
        let key = &rest[1..end];
        match args.iter().find(|(name, _)| *name == key) {
            Some((_, value)) => result.push_str(value),
            None => result.push_str(&rest[..=end]),
        }
        rest = &rest[end + 1..];
    }
    result.push_str(rest);
    result
}

/// Commands keep their engine labels in history. Translate their known
/// display templates here, keeping user-provided names literally intact.
pub(crate) fn tr_history(source: &str) -> String {
    let translated = tr(source);
    if translated != source {
        return translated;
    }
    for (prefix, suffix, template, field, localize_value) in [
        ("Add layer '", "'", "Add layer '{name}'", "name", false),
        ("Add ", " layer", "Add {kind} layer", "kind", true),
        ("Edit smart filter ", "", "Edit smart filter {kind}", "kind", true),
        ("Edit ", " shape", "Edit {kind} shape", "kind", true),
        ("Edit ", "", "Edit {kind}", "kind", true),
        ("New ", " shape", "New {kind} shape", "kind", true),
        ("Blend mode: ", "", "Blend mode: {mode}", "mode", true),
        ("Feather ", " px", "Feather {radius} px", "radius", false),
        ("Save path \"", "\"", "Save path \"{name}\"", "name", false),
        ("Snapshot: ", "", "Snapshot: {name}", "name", false),
        ("Smart filter: ", "", "Smart filter: {kind}", "kind", true),
        ("Layer mask: ", "", "Layer mask: {kind}", "kind", true),
        ("Lock ", "", "Lock {kind}", "kind", true),
        ("Align ", "", "Align {kind}", "kind", true),
        ("Distribute ", "", "Distribute {kind}", "kind", true),
        ("Rotate canvas ", "°", "Rotate canvas {angle}°", "angle", false),
    ] {
        if let Some(value) = source.strip_prefix(prefix).and_then(|s| s.strip_suffix(suffix)) {
            let value = if localize_value {
                tr(value)
            } else {
                value.to_owned()
            };
            return tr_args(template, &[(field, value)]);
        }
    }
    tr(source)
}

pub(crate) fn tr_status(source: &str) -> String {
    for (prefix, template) in [("Undid ", "Undid {action}"), ("Redid ", "Redid {action}")] {
        if let Some(action) = source.strip_prefix(prefix) {
            return tr_args(template, &[("action", tr_history(action))]);
        }
    }
    if let Some(detail) = source.strip_prefix("Could not ") {
        return tr_args("Could not {detail}", &[("detail", detail.to_owned())]);
    }
    tr_history(source)
}

impl crate::App {
    pub(crate) fn set_language(&mut self, ctx: &eframe::egui::Context, language: Language) {
        self.prefs.language = language;
        // Preferences holds a draft: applying it must keep the latest choice.
        if let Some(crate::Dialog::Preferences(p, _)) = self.dialog.as_mut() {
            p.language = language;
        }
        self.prefs.save();
        set_current(language);
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{self, Event, PointerButton, Pos2};

    #[test]
    fn preferences_migrate_and_persist_the_language() {
        let legacy: crate::session::Prefs = serde_json::from_str(r#"{"undo_steps":42}"#).unwrap();
        assert_eq!(legacy.language, Language::English);
        assert_eq!(legacy.undo_steps, 42);
        let chinese = crate::session::Prefs {
            language: Language::Chinese,
            ..legacy
        };
        let saved = serde_json::to_string(&chinese).unwrap();
        assert!(saved.contains(r#""language":"zh-CN""#));
        let restored: crate::session::Prefs = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.language, Language::Chinese);
        assert_eq!(restored.undo_steps, 42);
        let future: crate::session::Prefs =
            serde_json::from_str(r#"{"language":"future","undo_steps":42}"#).unwrap();
        assert_eq!(future.language, Language::English);
        assert_eq!(future.undo_steps, 42);
    }

    #[test]
    fn scopes_restore_language_and_values_are_inserted_literally() {
        assert_eq!(tr("File"), "File");
        {
            let _language = Language::Chinese.enter();
            assert_eq!(tr("File"), "文件");
            assert_eq!(tr("CURVES"), "曲线");
            assert_eq!(tr_history("Add layer 'Normal {name}'"), "添加图层“Normal {name}”");
            assert_eq!(tr_history("Lock transparent pixels"), "锁定透明像素");
            assert_eq!(tr_status("Undid Paint stroke"), "已撤销 绘画笔触");
            assert_eq!(tr("untranslated future text"), "untranslated future text");
            assert_eq!(
                tr_args("Undo {action}", &[("action", "file {action}.png".into())]),
                "撤销 file {action}.png"
            );
            let _english = Language::English.enter();
            assert_eq!(tr("File"), "File");
        }
        assert_eq!(tr("File"), "File");
    }

    fn frame(app: &mut crate::App, ctx: &egui::Context, events: Vec<Event>) -> Vec<String> {
        let out = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(1440.0, 900.0))),
                events,
                ..Default::default()
            },
            |ctx| app.frame(ctx),
        );
        out.shapes
            .into_iter()
            .filter_map(|s| match s.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                _ => None,
            })
            .collect()
    }

    fn click(app: &mut crate::App, ctx: &egui::Context, target: &str) {
        let pos = crate::theme::target_rect(ctx, target)
            .expect("target is displayed")
            .center();
        for pressed in [true, false] {
            frame(
                app,
                ctx,
                vec![
                    Event::PointerMoved(pos),
                    Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        frame(app, ctx, vec![]);
    }

    #[test]
    fn menu_switches_live_saves_and_keeps_document_data_and_action_ids() {
        let mut app = crate::a11y_tests::launch(&[]);
        app.prefs.language = Language::English;
        app.open_in_new_tab(crate::blank(64, 64), None);
        let ctx = crate::a11y_tests::ctx();
        let revision = app.editor.revision();
        let names: Vec<String> = app.editor.doc().layers().iter().map(|l| l.name.clone()).collect();
        frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, "Language");
        click(&mut app, &ctx, "简体中文");
        assert_eq!(app.prefs.language, Language::Chinese);
        assert_eq!(crate::session::Prefs::load().language, Language::Chinese);
        let texts = frame(&mut app, &ctx, vec![]);
        for text in ["文件", "编辑", "语言", "画笔", "不透明度", "图层", "无选区"] {
            assert!(texts.iter().any(|t| t == text), "{text} is displayed: {texts:?}");
        }
        app.dialog = Some(crate::Dialog::Preferences(app.prefs.clone(), None));
        let dialog_texts = frame(&mut app, &ctx, vec![]);
        assert!(dialog_texts.iter().any(|t| t == "偏好设置"));
        app.dialog = None;
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.editor.revision(), revision);
        assert_eq!(
            app.editor
                .doc()
                .layers()
                .iter()
                .map(|l| l.name.clone())
                .collect::<Vec<_>>(),
            names
        );
        // An existing command still uses its stable English id.
        app.run_action(&ctx, "new-layer");
        assert_eq!(app.editor.doc().layer_count(), names.len() + 1);
        click(&mut app, &ctx, "Language");
        click(&mut app, &ctx, "English");
        assert_eq!(app.prefs.language, Language::English);
        assert!(frame(&mut app, &ctx, vec![]).iter().any(|t| t == "File"));
    }

    #[test]
    fn every_chinese_catalogue_glyph_has_a_bundled_font() {
        let ctx = crate::a11y_tests::ctx();
        let _ = ctx.run(Default::default(), |ctx| {
            for family in [
                egui::FontFamily::Proportional,
                egui::FontFamily::Monospace,
                egui::FontFamily::Name("semibold".into()),
            ] {
                let font = egui::FontId::new(13.0, family);
                for text in Language::Chinese.catalogue().unwrap().values() {
                    for c in text.chars().filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c)) {
                        assert!(ctx.fonts(|f| f.has_glyph(&font, c)), "missing Chinese glyph {c}");
                    }
                }
            }
        });
    }
}
