use eframe::egui::{Align, Area, Color32, Frame, Id, Key, Layout, Order, Pos2, Stroke, Style, Ui, vec2};
use egui::{Popup, Rect, Response, Sense, Vec2, Widget};

pub fn popup_at<R>(ui: &Ui, popup_id: Id, pos: Pos2, width: f32, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    if Popup::is_id_open(ui.ctx(), popup_id) {
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
            Popup::close_id(ui.ctx(), popup_id);
        } else {
            ui.ctx().memory_mut(|mem| mem.keep_popup_open(popup_id));
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
pub struct ScrollBar<'a> {
    is_vertical: bool,
    // Position is 0..(len-visible_len)
    position: &'a mut f32,
    // Drag position is the offset from the center of the bar, if dragging is started from inside the bar
    // otherwise bar middle is set to clicked position on the whole bar
    drag_pos: &'a mut Option<f32>,
    // The whole length of virtual area to scroll
    len: f32,
    // The visible area to scroll
    visible_len: f32,
}

impl<'a> ScrollBar<'a> {
    pub fn new(position: &'a mut f32, drag_pos: &'a mut Option<f32>, len: f32, visible_len: f32) -> Self {
        ScrollBar {
            is_vertical: true,
            position,
            drag_pos,
            len,
            visible_len,
        }
    }
}

impl Widget for ScrollBar<'_> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let h = ui.available_height();
        let desired_size = Vec2::new(20.0, h); // Box size
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());

        let mut bar_len = (h * self.visible_len / self.len).max(20.0).min(h);
        let bar_pos = *self.position * (h - bar_len) / (self.len - self.visible_len);
        if bar_pos + bar_len > h {
            bar_len = h - bar_pos;
        }

        let bar_rec = Rect::from_min_size(
            Pos2::new(rect.min.x, rect.min.y + bar_pos),
            Vec2::new(rect.width(), bar_len),
        );

        if let Some(pointer_pos) = response.interact_pointer_pos() {
            // if drag_pos is none so the is the beginning of the click or drag operation
            if self.drag_pos.is_none() {
                let pointer_pos = if self.is_vertical {
                    pointer_pos.y - rect.min.y
                } else {
                    pointer_pos.x - rect.min.x
                };
                if pointer_pos < bar_pos || pointer_pos > bar_pos + bar_len {
                    // Clicked outer bar
                    *self.drag_pos = Some(0.0)
                } else {
                    // Clicked on bar so calculate the offset from the center
                    *self.drag_pos = Some(bar_pos + bar_len / 2.0 - pointer_pos);
                }
            }
            if let Some(drag_pos) = *self.drag_pos {
                let pointer_pos = if self.is_vertical {
                    pointer_pos.y - rect.min.y + drag_pos
                } else {
                    pointer_pos.x - rect.min.x + drag_pos
                };
                if pointer_pos < bar_len / 2.0 {
                    *self.position = 0.0;
                } else if pointer_pos > h - bar_len / 2.0 {
                    *self.position = self.len - self.visible_len;
                } else {
                    *self.position = (pointer_pos - bar_len / 2.0) * (self.len - self.visible_len) / (h - bar_len);
                }
            }
            if response.drag_stopped() {
                *self.drag_pos = None;
            }
        } else {
            if self.drag_pos.is_some() {
                *self.drag_pos = None;
            }
            let scroll = response.ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 && self.len > self.visible_len {
                *self.position -= scroll;
                *self.position = self.position.clamp(0.0, self.len - self.visible_len);
            }
        }

        // Draw the filled box
        ui.painter().rect_filled(rect, 5.0, ui.visuals().extreme_bg_color);

        ui.painter().rect_filled(bar_rec, 5.0, ui.visuals().text_color());

        response
    }
}

#[derive(Debug)]
pub struct ColorBox {
    color: Color32,
}

impl ColorBox {
    pub fn new(color: Color32) -> Self {
        ColorBox { color }
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

pub fn primary_color(visuals: &egui::Visuals) -> Color32 {
    if visuals.dark_mode {
        egui::Color32::DARK_GREEN
    } else {
        egui::Color32::LIGHT_GREEN
    }
}

#[cfg(target_arch = "wasm32")]
pub fn web_download(file_name: &str, data: &[u8]) -> Result<(), String> {
    // create blob
    use eframe::wasm_bindgen::JsCast;
    use js_sys::Array;

    let array_data = Array::new();
    array_data.push(&js_sys::Uint8Array::from(data));
    let blob = web_sys::Blob::new_with_u8_array_sequence(&array_data).map_err(|_| "Cannot create image data")?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| "Cannot create image url data")?;
    // create link
    let document = web_sys::window()
        .ok_or("Cannot get the website window")?
        .document()
        .ok_or("Cannot get the website document")?;
    let a = document.create_element("a").map_err(|_| "Cannot create <a> element")?;
    a.set_attribute("href", &url)
        .map_err(|_| "Cannot create add href attribute")?;
    a.set_attribute("download", file_name)
        .map_err(|_| "Cannot create add download attribute")?;

    // click link
    a.dyn_ref::<web_sys::HtmlElement>()
        .ok_or("Cannot simulate click")?
        .click();
    // revoke url
    web_sys::Url::revoke_object_url(&url).map_err(|_| "Cannot remove object url with revoke_object_url".into())
}
