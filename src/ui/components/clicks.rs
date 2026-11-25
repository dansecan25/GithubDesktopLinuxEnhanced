//! Click handling that tells left and right clicks apart reliably.
//!
//! Some laptop touchpads report a left click together with (or right after) a right click. Without
//! this, a right-click would also open a menu button's dropdown, select a row, or even switch
//! branch, on top of showing the right-click menu.

use eframe::egui::{self, PointerButton, Popup, Response, SetOpenCommand, Ui};

use crate::constants::layout::{CLICK_AFTER_RIGHT_CLICK_SECONDS, RIGHT_CLICK_MENU_MIN_WIDTH};

fn last_secondary_press_id() -> egui::Id {
    egui::Id::unique("last-secondary-press")
}

/// Call once at the start of every frame, before any widget is drawn.
pub fn record_secondary_presses(ctx: &egui::Context) {
    let (pressed, now) = ctx.input(|input| {
        let pressed =
            input.pointer.button_pressed(PointerButton::Secondary) || input.pointer.secondary_down();
        (pressed, input.time)
    });
    if pressed {
        ctx.data_mut(|data| data.insert_temp(last_secondary_press_id(), now));
    }
}

fn part_of_right_click(ctx: &egui::Context) -> bool {
    let now = ctx.input(|input| input.time);
    let last = ctx.data(|data| data.get_temp::<f64>(last_secondary_press_id()));
    last.is_some_and(|at| now - at < CLICK_AFTER_RIGHT_CLICK_SECONDS)
}

/// A left click that isn't part of a right-click.
pub fn left_clicked(response: &Response) -> bool {
    response.clicked() && !response.secondary_clicked() && !part_of_right_click(&response.ctx)
}

fn open_right_click_menu_id() -> egui::Id {
    egui::Id::unique("open-right-click-menu")
}

/// A right-click menu at the pointer.
///
/// It is drawn as its own floating layer instead of an egui popup: egui keeps one popup open at a
/// time, so a popup here would close the branch or repository dropdown it was opened from. Only one
/// right-click menu is open at a time. It closes on a menu item, Escape, or a real click elsewhere;
/// a left click that comes with the right click (some touchpads) doesn't close it.
pub fn right_click_menu(response: &Response, content: impl FnOnce(&mut Ui)) {
    let ctx = response.ctx.clone();
    let menu_id = response.id.with("right-click-menu");
    if response.secondary_clicked() {
        let at = ctx.input(|input| input.pointer.interact_pos()).unwrap_or(response.rect.center());
        ctx.data_mut(|data| data.insert_temp(open_right_click_menu_id(), (menu_id, at)));
    }
    let open = ctx.data(|data| data.get_temp::<(egui::Id, egui::Pos2)>(open_right_click_menu_id()));
    let Some((open_id, at)) = open else { return };
    if open_id != menu_id {
        return;
    }
    egui::Area::new(menu_id).order(egui::Order::Foreground).fixed_pos(at).constrain(true).show(&ctx, |ui| {
        egui::Frame::menu(ui.style()).show(ui, |ui| {
            ui.set_min_width(RIGHT_CLICK_MENU_MIN_WIDTH);
            egui::containers::menu::menu_style(ui.style_mut());
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), content);
        });
    });
    let (clicked, escape) =
        ctx.input(|input| (input.pointer.any_click(), input.key_pressed(egui::Key::Escape)));
    // A click on an item has already run its action this frame, so any real click closes the menu.
    if escape || (clicked && !response.secondary_clicked() && !part_of_right_click(&ctx)) {
        ctx.data_mut(|data| data.remove::<(egui::Id, egui::Pos2)>(open_right_click_menu_id()));
    }
}

/// A dropdown that opens on a left click only (see [`left_clicked`]).
pub fn dropdown_menu(button: &Response) -> Popup<'_> {
    Popup::menu(button).open_memory(left_clicked(button).then_some(SetOpenCommand::Toggle))
}
