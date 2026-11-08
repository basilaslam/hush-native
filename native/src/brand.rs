use eframe::egui;

const SIDE: usize = 256;
const PIXELS: &[u8; SIDE * SIDE * 4] = include_bytes!("../assets/icon.rgba");

pub fn window_icon() -> egui::IconData {
    egui::IconData {
        rgba: PIXELS.to_vec(),
        width: SIDE as u32,
        height: SIDE as u32,
    }
}

// Upload once per egui context. Painting the brand has no decoding or per-frame upload.
pub fn texture(ctx: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::new("hush-brand-texture");
    if let Some(texture) = ctx.data(|data| data.get_temp::<egui::TextureHandle>(id)) {
        return texture;
    }
    let texture = ctx.load_texture(
        "Hush",
        egui::ColorImage::from_rgba_unmultiplied([SIDE, SIDE], PIXELS),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|data| data.insert_temp(id, texture.clone()));
    texture
}
