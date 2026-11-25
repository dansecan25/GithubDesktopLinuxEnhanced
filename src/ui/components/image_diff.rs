//! An image change shown as pictures, like GitHub Desktop's "2-up" view: before on the left (red
//! border), after on the right (green border), or one above the other when the pane is narrow.

use eframe::egui::{self, Color32, RichText, Ui};

use super::messages::muted_text;
use crate::constants::layout::{CHECKER_SIZE, IMAGE_DIFF_SIDE_BY_SIDE_WIDTH};
use crate::constants::text;
use crate::models::{ImagePair, ImageSide, PreviewImage};
use crate::ui::theme;

pub fn image_diff(ui: &mut Ui, pair: &ImagePair, id: &str) {
    let palette = theme::palette(ui.visuals());
    let both = !matches!(pair.before, ImageSide::Missing) && !matches!(pair.after, ImageSide::Missing);
    let sides: Vec<(&ImageSide, &str, Color32, &str)> = if both {
        vec![
            (&pair.before, text::IMAGE_BEFORE, palette.removed, "before"),
            (&pair.after, text::IMAGE_AFTER, palette.added, "after"),
        ]
    } else if matches!(pair.after, ImageSide::Missing) {
        vec![(&pair.before, text::IMAGE_DELETED, palette.removed, "before")]
    } else {
        vec![(&pair.after, text::IMAGE_ADDED, palette.added, "after")]
    };
    egui::ScrollArea::both().id_salt(("image-diff", id)).auto_shrink([false, false]).show(ui, |ui| {
        let side_by_side = sides.len() == 2 && ui.available_width() >= IMAGE_DIFF_SIDE_BY_SIDE_WIDTH;
        if side_by_side {
            let width = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
            ui.horizontal_top(|ui| {
                for (side, label, color, key) in &sides {
                    ui.allocate_ui(egui::vec2(width, ui.available_height()), |ui| {
                        ui.vertical(|ui| show_side(ui, side, label, *color, &format!("{id}-{key}"), width));
                    });
                }
            });
        } else {
            let width = ui.available_width();
            for (side, label, color, key) in &sides {
                show_side(ui, side, label, *color, &format!("{id}-{key}"), width);
                ui.add_space(12.0);
            }
        }
    });
}

fn show_side(ui: &mut Ui, side: &ImageSide, label: &str, border: Color32, key: &str, width: f32) {
    ui.label(RichText::new(label).strong());
    match side {
        ImageSide::Image(image) => {
            ui.label(muted_text(ui, &text::image_details(image.width, image.height, image.file_size)));
            picture(ui, image, border, key, width);
        }
        ImageSide::Unavailable(reason) => {
            ui.label(muted_text(ui, reason));
        }
        ImageSide::Missing => {}
    }
}

/// The image at its real size, or smaller to fit, on a checkerboard so transparency shows.
fn picture(ui: &mut Ui, image: &PreviewImage, border: Color32, key: &str, width: f32) {
    let texture = texture(ui, image, key);
    let real = egui::vec2(image.width as f32, image.height as f32);
    let scale = (width / real.x).min(1.0);
    let size = real * scale;
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    checkerboard(ui, rect);
    ui.painter().image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    ui.painter().rect_stroke(rect, 0.0, egui::Stroke::new(1.0, border), egui::StrokeKind::Outside);
}

/// One texture per side, made again only when a new image is decoded.
fn texture(ui: &Ui, image: &PreviewImage, key: &str) -> egui::TextureHandle {
    let id = egui::Id::unique(("image-diff-texture", key));
    let identity = std::sync::Arc::as_ptr(&image.rgba) as usize;
    if let Some((cached, handle)) = ui.data(|data| data.get_temp::<(usize, egui::TextureHandle)>(id))
        && cached == identity
    {
        return handle;
    }
    let size = [image.shown_width as usize, image.shown_height as usize];
    let pixels = egui::ColorImage::from_rgba_unmultiplied(size, &image.rgba);
    let handle = ui.ctx().load_texture(key, pixels, egui::TextureOptions::LINEAR);
    ui.data_mut(|data| data.insert_temp(id, (identity, handle.clone())));
    handle
}

fn checkerboard(ui: &Ui, rect: egui::Rect) {
    let painter = ui.painter().with_clip_rect(rect);
    let (light, dark) = (Color32::from_gray(200), Color32::from_gray(160));
    painter.rect_filled(rect, 0.0, light);
    let columns = (rect.width() / CHECKER_SIZE).ceil() as usize;
    let rows = (rect.height() / CHECKER_SIZE).ceil() as usize;
    for row in 0..rows {
        for column in (row % 2..columns).step_by(2) {
            let min = rect.min + egui::vec2(column as f32, row as f32) * CHECKER_SIZE;
            painter.rect_filled(egui::Rect::from_min_size(min, egui::Vec2::splat(CHECKER_SIZE)), 0.0, dark);
        }
    }
}
