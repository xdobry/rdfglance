use eframe::egui::{
    vec2, Align, Area, Color32, Frame, Id, Key, Layout, Order, Pos2, Stroke, Style, Ui,
};
use egui::{Response, Sense, Vec2, Widget};

pub fn popup_at<R>(
    ui: &Ui,
    popup_id: Id,
    pos: Pos2,
    width: f32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    if ui.memory(|mem| mem.is_popup_open(popup_id)) {
        let inner = Area::new(popup_id)
            .order(Order::Foreground)
            .constrain(true)
            .fixed_pos(pos)
            .show(ui.ctx(), |ui| {
                let frame = Frame::popup(ui.style());
                set_menu_style(ui.style_mut());
                frame
                    .show(ui, |ui| {
                        ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                            ui.set_width(width);
                            add_contents(ui)
                        })
                        .inner
                    })
                    .inner
            })
            .inner;

        if ui.input(|i| i.key_pressed(Key::Escape)) {
            ui.memory_mut(|mem| mem.close_popup());
        }
        Some(inner)
    } else {
        None
    }
}

fn set_menu_style(style: &mut Style) {
    style.spacing.button_padding = vec2(2.0, 0.0);
    style.visuals.widgets.active.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
}

pub fn strong_unselectable(ui: &mut Ui, text: impl Into<egui::RichText>) {
    let l = egui::Label::new(text.into().strong()).selectable(false);
    ui.add(l);
}

#[derive(Debug)]
pub struct ColorBox {
    color: Color32,
}

impl ColorBox {
    pub fn new(color: Color32) -> Self {
        ColorBox {
            color
        }
    }
}

impl Widget for ColorBox {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let desired_size = Vec2::new(20.0, 17.0); // Box size
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::empty());

        ui.painter().rect_filled(rect, 3.0, self.color);

        response
    }
}

const APP_ICON: &[u8] = include_bytes!("../assets/rdfglance-icon.ico");

pub fn load_icon() -> eframe::egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(APP_ICON)
            .expect("Failed to open icon path")
            .into_rgba8();

        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    eframe::egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}
