//! Text measurements use the same default fonts as native egui painting.
use std::sync::OnceLock;

pub(crate) fn size(text: &str, size: f32) -> [f32; 2] {
    static CONTEXT: OnceLock<egui::Context> = OnceLock::new();
    let ctx = CONTEXT.get_or_init(|| {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        ctx
    });
    ctx.fonts_mut(|fonts| {
        let size = fonts
            .layout_no_wrap(
                text.into(),
                egui::FontId::proportional(size),
                egui::Color32::BLACK,
            )
            .size();
        [size.x, size.y]
    })
}

pub(crate) fn width(text: &str, size: f32) -> f32 {
    self::size(text, size)[0]
}
