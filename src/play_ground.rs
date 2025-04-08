use egui::{Color32, Pos2, Rect, Response, Sense, Vec2, Widget};
use egui_extras::StripBuilder;

use crate::{NodeAction, RdfGlanceApp};

#[derive(Debug)]
pub struct ScrollBar<'a> {
    is_vertical: bool,
    // Position is 0..(len-visible_len)
    position: &'a mut f32,
    // Drag position is the offset from the center of the bar, if dragging is started from inside the bar
    // otherwise bar middle is set to clicked possition on the whole bar
    drag_pos: &'a mut Option<f32>,
    // The whole length of virtual area to scroll
    len: f32,
    // The visible area to scroll
    visible_len: f32,
}

impl<'a> ScrollBar<'a> {
    pub fn new(position: &'a mut f32,drag_pos: &'a mut Option<f32>, len: f32, visible_len: f32) -> Self {
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
                    *self.drag_pos = Some(bar_pos + bar_len/2.0 - pointer_pos);
                }
            }
            if let Some(drag_pos) = *self.drag_pos {
                let pointer_pos = if self.is_vertical {
                    pointer_pos.y - rect.min.y + drag_pos
                } else {
                    pointer_pos.x - rect.min.x + drag_pos
                };
                if pointer_pos < bar_len/2.0 {
                    *self.position = 0.0;
                } else if pointer_pos > h - bar_len/2.0 {
                    *self.position = self.len - self.visible_len;
                } else {
                    *self.position = (pointer_pos - bar_len/2.0) * (self.len - self.visible_len) / (h - bar_len);
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
            if scroll!=0.0 && self.len>self.visible_len {
                *self.position -= scroll;
                *self.position = self.position.clamp(0.0, self.len - self.visible_len);
            }

        }


        // Draw the filled box
        ui.painter()
            .rect_filled(rect, 5.0, Color32::LIGHT_GRAY); // 5.0 is corner rounding

        ui.painter().rect_filled(bar_rec, 5.0, Color32::DARK_GRAY);

        response
    }
}

impl RdfGlanceApp {
    pub fn show_play(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        let node_to_click: NodeAction = NodeAction::None;

        /*
        CentralPanel::default().show(ctx, |ui| {
            let available_height = ui.available_height();

            ScrollArea::vertical().show(ui, |ui| {
                // Calculate total required height
                let total_height = self.play.row_height * self.play.row_count as f32;
                let (response, painter) = ui.allocate_painter(
                    egui::vec2(ui.available_width(), total_height),
                    egui::Sense::hover(),
                );

                let clip_rect = response.rect.intersect(ui.clip_rect());
                let start_index = (clip_rect.min.y / self.play.row_height).floor() as usize;
                let end_index = ((clip_rect.max.y / self.play.row_height).ceil() as usize)
                    .min(self.play.row_count);

                for i in start_index..end_index {
                    let row_y = i as f32 * self.play.row_height;
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(response.rect.min.x, row_y),
                        egui::vec2(response.rect.width(), self.play.row_height),
                    );

                    // Draw row background (optional)
                    if i % 2 == 0 {
                        painter.rect_filled(rect, 0.0, egui::Color32::from_gray(30));
                    }

                    // Draw text
                    painter.text(
                        rect.left_center(),
                        egui::Align2::LEFT_CENTER,
                        format!("Row {} {} {}", i, clip_rect.min.y,start_index),
                        egui::TextStyle::Body.resolve(ui.style()),
                        egui::Color32::WHITE,
                    );
                }
            });
        });
        */

        // Wee need to add one row_height to ensure that all rows can be desplayed
        let needed_len = self.play.row_count as f32 * self.play.row_height + self.play.row_height;

        ui.label(format!("Position: {} len: {}", self.play.position, needed_len));

        let a_height = ui.available_height();
        let capacity = (a_height / self.play.row_height) as usize;
        let item_pos = (self.play.position/self.play.row_height) as usize;
        /*
        if self.play.position > self.play.row_count - capacity {
            self.play.position = self.play.row_count - capacity;
        }
        */
        // println!("needed_len: {} a_height: {} Capacity: {} sb_pos: {} item_pos: {} row_height: {}", needed_len, a_height, capacity, self.play.position, item_pos, self.play.row_height);
        let mut first = true;

        StripBuilder::new(ui)
            .size(egui_extras::Size::remainder())
            .size(egui_extras::Size::exact(20.0)) // Two resizable panels with equal initial width
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    for i in item_pos..item_pos+capacity {
                        let response = ui.label(format!("Row {}", i));
                        if first {
                            self.play.row_height = response.rect.height() + ui.spacing().item_spacing.y;
                            first = false;
                        }
                    }
                });
                strip.cell(|ui| {
                    ui.add(ScrollBar::new(&mut self.play.position,&mut self.play.drag_pos, needed_len,a_height));
                });
            });
        node_to_click
    }
}
