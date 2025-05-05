
use egui::{Color32, RichText, Sense, Slider, Vec2};

use crate::{drawing::{draw_edge, draw_node_label}, nobject::{IriIndex, LabelContext}, RdfGlanceApp, StyleEdit};

pub struct NodeStyle {
    pub color: egui::Color32,
    pub priority: u32,
    pub label_index: IriIndex,
    pub node_shape: NodeShape,
    pub node_size: NodeSize,
    pub width: f32,
    pub height: f32,
    pub border_width: f32,
    pub border_color: egui::Color32,
    pub corner_radius: f32,
    pub max_lines: u16,
    pub label_position: LabelPosition,
    pub label_max_width: f32,
    pub font_size: f32,
    pub label_color: egui::Color32,
    pub icon_style: Option<IconStyle>,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            color: egui::Color32::WHITE,
            priority: 0,
            label_index: 0,
            node_shape: NodeShape::Circle,
            node_size: NodeSize::Fixed,
            width: 10.0,
            height: 10.0,
            border_width: 1.0,
            border_color: egui::Color32::BLACK,
            corner_radius: 3.0,
            max_lines: 1,
            label_position: LabelPosition::Above,
            label_max_width: 0.0,
            font_size: 16.0,
            label_color: egui::Color32::BLACK,
            icon_style: None,
        }
    }
}

pub struct IconStyle {
    pub icon_character: char,
    pub icon_position: IconPosition,
    pub icon_size: f32,
    pub icon_color: egui::Color32,
}

impl Default for IconStyle {
    fn default() -> Self {
        Self {
            icon_character: '\u{2606}',
            icon_position: IconPosition::Center,
            icon_size: 20.0,
            icon_color: Color32::BLACK,
        }
    }
}

pub struct EdgeFont {
    pub font_size: f32,
    pub font_color: Color32,
}

impl Default for EdgeFont {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            font_color: Color32::BLACK,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum NodeShape {
    None = 0,
    Rect = 1,
    Circle = 2,
    Elipse = 3,
}

impl TryFrom<u8> for NodeShape {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(NodeShape::None),
            1 => Ok(NodeShape::Rect),
            2 => Ok(NodeShape::Circle),
            3 => Ok(NodeShape::Elipse),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum NodeSize {
    Fixed = 1,
    Label = 2,   
}

impl TryFrom<u8> for NodeSize {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(NodeSize::Fixed),
            2 => Ok(NodeSize::Label),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum LabelPosition {
    Center = 1,
    Above = 2,
    Below = 3,
    Right = 4,
    Left = 5,  
}

impl TryFrom<u8> for LabelPosition {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(LabelPosition::Center),
            2 => Ok(LabelPosition::Above),
            3 => Ok(LabelPosition::Below),
            4 => Ok(LabelPosition::Right),
            5 => Ok(LabelPosition::Left),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum IconPosition {
    Center = 1,
    Above = 2,
    Below = 3,
    Right = 4,
    Left = 5,  
}

impl TryFrom<u8> for IconPosition {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(IconPosition::Center),
            2 => Ok(IconPosition::Above),
            3 => Ok(IconPosition::Below),
            4 => Ok(IconPosition::Right),
            5 => Ok(IconPosition::Left),
            _ => Err(()),
        }
    }
}

pub struct EdgeStyle {
    pub color: egui::Color32,
    pub width: f32,
    pub line_gap: f32,
    pub line_style: LineStyle,
    pub target_style: ArrowStyle,
    pub arrow_location: ArrowLocation,
    pub arrow_size: f32,
    pub icon_style: Option<IconStyle>,
    pub edge_font: Option<EdgeFont>,
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self {
            color: egui::Color32::BLACK,
            width: 2.0,
            icon_style: None,
            edge_font: None,
            line_style: LineStyle::Solid,
            target_style: ArrowStyle::Arrow,
            arrow_location: ArrowLocation::Target,
            line_gap: 10.0,
            arrow_size: 6.0,
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

impl TryFrom<u8> for LineStyle {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(LineStyle::Solid),
            1 => Ok(LineStyle::Dashed),
            2 => Ok(LineStyle::Dotted),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum ArrowStyle {
    Arrow = 0,
    ArrorFilled = 1,
    ArrorTriangle = 2,
}

impl TryFrom<u8> for ArrowStyle {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ArrowStyle::Arrow),
            1 => Ok(ArrowStyle::ArrorFilled),
            2 => Ok(ArrowStyle::ArrorTriangle),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum ArrowLocation {
    Target = 0,
    Middle = 1,
    None = 2,
}

impl TryFrom<u8> for ArrowLocation {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ArrowLocation::Target),
            1 => Ok(ArrowLocation::Middle),
            2 => Ok(ArrowLocation::None),
            _ => Err(()),
        }
    }
}

impl RdfGlanceApp {
    pub fn display_node_style(&mut self, ui: &mut egui::Ui, type_style_edit: IriIndex) {
        let type_style = self.visualisation_style.node_styles.get_mut(&type_style_edit);
        if let Some(type_style) = type_style {
            let label_context = LabelContext::new(self.ui_state.display_language, self.persistent_data.config_data.iri_display, &self.prefix_manager);
            let type_label = self.node_data.type_display(
                type_style_edit,
                &label_context,
                &self.node_data.indexers
            );
            ui.heading(format!("Node Style for Type: {}",type_label.as_str()));
            if ui.button("Close Style Edit").clicked() {
                self.ui_state.style_edit = StyleEdit::None;
            }
            ui.horizontal(|ui| {
                ui.label("Priority:");
                ui.add(Slider::new(&mut type_style.priority, 0..=1000));
            });
            ui.horizontal(|ui| {
                ui.label("Background Color:");
                ui.color_edit_button_srgba(&mut type_style.color);
            });
            ui.horizontal(|ui| {
                ui.label("Shape:");
                ui.selectable_value(&mut type_style.node_shape, NodeShape::Circle, "Circle");
                ui.selectable_value(&mut type_style.node_shape, NodeShape::Rect, "Rectangle");
                ui.selectable_value(&mut type_style.node_shape, NodeShape::None, "No Shape");
                // ui.selectable_value(&mut type_style.node_shape, NodeShape::Elipse, "Ellipse");
            });
            ui.horizontal(|ui| {
                ui.label("Rectangle Corner Radius:");
                ui.add(Slider::new(&mut type_style.corner_radius, 0.0..=20.0));
            });
            ui.horizontal(|ui| {
                ui.label("Sizing:");
                ui.selectable_value(&mut type_style.node_size, NodeSize::Fixed, "Fixed");
                ui.selectable_value(&mut type_style.node_size, NodeSize::Label, "Label Dependant");
            });
            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(Slider::new(&mut type_style.width, 3.0..=150.0));
            });
            ui.horizontal(|ui| {
                ui.label("Height:");
                ui.add(Slider::new(&mut type_style.height, 3.0..=150.0));
            });
            ui.horizontal(|ui| {
                ui.label("Border Width:");
                ui.add(Slider::new(&mut type_style.border_width, 0.0..=20.0));
            });
            ui.horizontal(|ui| {
                ui.label("Border Color:");
                ui.color_edit_button_srgba(&mut type_style.border_color);
            });
            ui.horizontal(|ui| {
                ui.label("Max Lines:");
                ui.add(Slider::new(&mut type_style.max_lines, 1..=10));
            });
            ui.horizontal(|ui| {
                ui.label("Font Size:");
                ui.add(Slider::new(&mut type_style.font_size, 5.0..=25.0));
            });
            ui.horizontal(|ui| {
                ui.label("Label Position:");
                ui.selectable_value(&mut type_style.label_position, LabelPosition::Center, "Center");
                ui.selectable_value(&mut type_style.label_position, LabelPosition::Above, "Above");
                ui.selectable_value(&mut type_style.label_position, LabelPosition::Below, "Below");
                ui.selectable_value(&mut type_style.label_position, LabelPosition::Right, "Right");
                ui.selectable_value(&mut type_style.label_position, LabelPosition::Left, "Left");
            });
            ui.horizontal(|ui| {
                ui.label("Label Color:");
                ui.color_edit_button_srgba(&mut type_style.label_color);
            });
            ui.horizontal(|ui| {
                ui.label("Label Max Width (0-unlimited):");
                ui.add(Slider::new(&mut type_style.label_max_width, 0.0..=300.0));
            });
            display_icon_style(ui, &mut type_style.icon_style, &mut self.ui_state.icon_name_filter);
            let desired_size = Vec2::new(800.0, 300.0); // width, height
            let (response, painter) = ui.allocate_painter(desired_size, Sense::empty());
            let node_label = "Test Label";
            draw_node_label(&painter, node_label, type_style, response.rect.center(), false, false, true);
        }
    }

    pub fn display_edge_style(&mut self, ui: &mut egui::Ui, edge_style_edit: IriIndex) {
        let edge_style = self.visualisation_style.edge_styles.get_mut(&edge_style_edit);
        if let Some(edge_style) = edge_style {
            let label_context = LabelContext::new(self.ui_state.display_language, self.persistent_data.config_data.iri_display, &self.prefix_manager);
            let predicate_label = self.node_data.predicate_display(
                edge_style_edit,
                &label_context,
                &self.node_data.indexers
            );
            ui.heading(format!("Edge Style for: {}",predicate_label.as_str()));
            if ui.button("Close Style Edit").clicked() {
                self.ui_state.style_edit = StyleEdit::None;
            }
            ui.horizontal(|ui| {
                ui.label("Color:");
                ui.color_edit_button_srgba(&mut edge_style.color);
            });
            ui.horizontal(|ui| {
                ui.label("Width:");
                ui.add(Slider::new(&mut edge_style.width, 1.0..=10.0));
            });
            ui.horizontal(|ui| {
                ui.label("Line Style:");
                ui.selectable_value(&mut edge_style.line_style, LineStyle::Solid, "Solid");
                ui.selectable_value(&mut edge_style.line_style, LineStyle::Dotted, "Dotted");
                ui.selectable_value(&mut edge_style.line_style, LineStyle::Dashed, "Dashed");
            });
            if !matches!(edge_style.line_style, LineStyle::Solid) {
                ui.horizontal(|ui| {
                    ui.label("Line Gap:");
                    ui.add(Slider::new(&mut edge_style.line_gap, 2.0..=20.0));
                });
            }
            ui.horizontal(|ui| {
                ui.label("Arrow Location:");
                ui.selectable_value(&mut edge_style.arrow_location, ArrowLocation::Target, "Target");
                ui.selectable_value(&mut edge_style.arrow_location, ArrowLocation::Middle, "Middle");
                ui.selectable_value(&mut edge_style.arrow_location, ArrowLocation::None, "None");
            });
            if !matches!(edge_style.arrow_location, ArrowLocation::None) {
                ui.horizontal(|ui| {
                    ui.label("Arrow Size:");
                    ui.add(Slider::new(&mut edge_style.arrow_size, 2.0..=40.0));
                });
            }
            ui.horizontal(|ui| {
                ui.label("Arrow Style");
                ui.selectable_value(&mut edge_style.target_style, ArrowStyle::Arrow, "Arrow");
                ui.selectable_value(&mut edge_style.target_style, ArrowStyle::ArrorFilled, "Filled Triangle");
                ui.selectable_value(&mut edge_style.target_style, ArrowStyle::ArrorTriangle, "Triangle");
            });
            if  edge_style.edge_font.is_some() {
                if ui.button("Clear Label").clicked() {
                    edge_style.edge_font = None;
                }
                if let Some(edge_font) = &mut edge_style.edge_font {
                    ui.horizontal(|ui| {
                        ui.label("Font Size:");
                        ui.add(Slider::new(&mut edge_font.font_size, 5.0..=25.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Font Color:");
                        ui.color_edit_button_srgba(&mut edge_font.font_color);
                    });
                }
            } else if ui.button("Add Label").clicked() {
                edge_style.edge_font = Some(EdgeFont::default());
            }

            display_icon_style(ui, &mut edge_style.icon_style, &mut self.ui_state.icon_name_filter);

            let desired_size = Vec2::new(800.0, 80.0); // width, height
            let (response, painter) = ui.allocate_painter(desired_size, Sense::empty());
            let node_label = || {
                String::from("Test Label")
            };
            draw_edge(&painter, 
            response.rect.min+Vec2::new(30.0,20.0), 
                Vec2::new(5.0,5.0),
                NodeShape::Circle,
                response.rect.min+Vec2::new(200.0,50.0),
                Vec2::new(5.0,5.0),
                NodeShape::Circle,
                &edge_style,
                node_label
            );
        }
    }
}

fn display_icon_style(ui: &mut egui::Ui, icon_style: &mut Option<IconStyle>, icon_name_fitler: &mut String) {
    if icon_style.is_some() {
        ui.heading("Additional Icon:");
        if ui.button("Clear icon").clicked() {
            *icon_style = None;
        }
        if let Some(icon_style_val) = icon_style {
            ui.horizontal(|ui| {
                ui.label("Icon location:");
                ui.selectable_value(&mut icon_style_val.icon_position, IconPosition::Center, "Center");
                ui.selectable_value(&mut icon_style_val.icon_position, IconPosition::Above, "Above");
                ui.selectable_value(&mut icon_style_val.icon_position, IconPosition::Below, "Below");
                ui.selectable_value(&mut icon_style_val.icon_position, IconPosition::Right, "Right");
                ui.selectable_value(&mut icon_style_val.icon_position, IconPosition::Left, "Left");
            });
            icon_edit_button(ui, &mut icon_style_val.icon_character, icon_name_fitler);
            ui.horizontal(|ui| {
                ui.label("Icon Size:");
                ui.add(Slider::new(&mut icon_style_val.icon_size, 5.0..=80.0));
            });
            ui.horizontal(|ui| {
                ui.label("Icon Color:");
                ui.color_edit_button_srgba(&mut icon_style_val.icon_color);
            });
        }
    } else {
        if ui.button("Add additional Icon").clicked() {
            *icon_style = Some(IconStyle {
                icon_character: '\u{2606}',
                icon_position: IconPosition::Center,
                icon_size: 20.0,
                icon_color: Color32::BLACK,
            });
        }
    }

}


// Chosing icon character
// Code partly from egui demo
// https://github.com/emilk/egui/blob/master/crates/egui_demo_lib/src/demo/font_book.rs

fn available_characters(ui: &egui::Ui, family: egui::FontFamily) -> Vec<(char, String)> {
    ui.fonts(|f| {
        f.lock()
            .fonts
            .font(&egui::FontId::new(10.0, family)) // size is arbitrary for getting the characters
            .characters()
            .iter()
            .filter(|(chr, _fonts)| !chr.is_whitespace() && !chr.is_ascii_control())
            .map(|(chr, _fonts)| {
                (
                    *chr,
                    char_name(*chr),
                )
            })
            .collect()
    })
}

fn char_name(chr: char) -> String {
    special_char_name(chr)
        .map(|s| s.to_owned())
        .or_else(|| unicode_names2::name(chr).map(|name| name.to_string().to_lowercase()))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn special_char_name(chr: char) -> Option<&'static str> {
    #[expect(clippy::match_same_arms)] // many "flag"
    match chr {
        // Special private-use-area extensions found in `emoji-icon-font.ttf`:
        // Private use area extensions:
        '\u{FE4E5}' => Some("flag japan"),
        '\u{FE4E6}' => Some("flag usa"),
        '\u{FE4E7}' => Some("flag"),
        '\u{FE4E8}' => Some("flag"),
        '\u{FE4E9}' => Some("flag"),
        '\u{FE4EA}' => Some("flag great britain"),
        '\u{FE4EB}' => Some("flag"),
        '\u{FE4EC}' => Some("flag"),
        '\u{FE4ED}' => Some("flag"),
        '\u{FE4EE}' => Some("flag south korea"),
        '\u{FE82C}' => Some("number sign in square"),
        '\u{FE82E}' => Some("digit one in square"),
        '\u{FE82F}' => Some("digit two in square"),
        '\u{FE830}' => Some("digit three in square"),
        '\u{FE831}' => Some("digit four in square"),
        '\u{FE832}' => Some("digit five in square"),
        '\u{FE833}' => Some("digit six in square"),
        '\u{FE834}' => Some("digit seven in square"),
        '\u{FE835}' => Some("digit eight in square"),
        '\u{FE836}' => Some("digit nine in square"),
        '\u{FE837}' => Some("digit zero in square"),

        // Special private-use-area extensions found in `emoji-icon-font.ttf`:
        // Web services / operating systems / browsers
        '\u{E600}' => Some("web-dribbble"),
        '\u{E601}' => Some("web-stackoverflow"),
        '\u{E602}' => Some("web-vimeo"),
        '\u{E604}' => Some("web-facebook"),
        '\u{E605}' => Some("web-googleplus"),
        '\u{E606}' => Some("web-pinterest"),
        '\u{E607}' => Some("web-tumblr"),
        '\u{E608}' => Some("web-linkedin"),
        '\u{E60A}' => Some("web-stumbleupon"),
        '\u{E60B}' => Some("web-lastfm"),
        '\u{E60C}' => Some("web-rdio"),
        '\u{E60D}' => Some("web-spotify"),
        '\u{E60E}' => Some("web-qq"),
        '\u{E60F}' => Some("web-instagram"),
        '\u{E610}' => Some("web-dropbox"),
        '\u{E611}' => Some("web-evernote"),
        '\u{E612}' => Some("web-flattr"),
        '\u{E613}' => Some("web-skype"),
        '\u{E614}' => Some("web-renren"),
        '\u{E615}' => Some("web-sina-weibo"),
        '\u{E616}' => Some("web-paypal"),
        '\u{E617}' => Some("web-picasa"),
        '\u{E618}' => Some("os-android"),
        '\u{E619}' => Some("web-mixi"),
        '\u{E61A}' => Some("web-behance"),
        '\u{E61B}' => Some("web-circles"),
        '\u{E61C}' => Some("web-vk"),
        '\u{E61D}' => Some("web-smashing"),
        '\u{E61E}' => Some("web-forrst"),
        '\u{E61F}' => Some("os-windows"),
        '\u{E620}' => Some("web-flickr"),
        '\u{E621}' => Some("web-picassa"),
        '\u{E622}' => Some("web-deviantart"),
        '\u{E623}' => Some("web-steam"),
        '\u{E624}' => Some("web-github"),
        '\u{E625}' => Some("web-git"),
        '\u{E626}' => Some("web-blogger"),
        '\u{E627}' => Some("web-soundcloud"),
        '\u{E628}' => Some("web-reddit"),
        '\u{E629}' => Some("web-delicious"),
        '\u{E62A}' => Some("browser-chrome"),
        '\u{E62B}' => Some("browser-firefox"),
        '\u{E62C}' => Some("browser-ie"),
        '\u{E62D}' => Some("browser-opera"),
        '\u{E62E}' => Some("browser-safari"),
        '\u{E62F}' => Some("web-google-drive"),
        '\u{E630}' => Some("web-wordpress"),
        '\u{E631}' => Some("web-joomla"),
        '\u{E632}' => Some("lastfm"),
        '\u{E633}' => Some("web-foursquare"),
        '\u{E634}' => Some("web-yelp"),
        '\u{E635}' => Some("web-drupal"),
        '\u{E636}' => Some("youtube"),
        '\u{F189}' => Some("vk"),
        '\u{F1A6}' => Some("digg"),
        '\u{F1CA}' => Some("web-vine"),
        '\u{F8FF}' => Some("os-apple"),

        // Special private-use-area extensions found in `Ubuntu-Light.ttf`
        '\u{F000}' => Some("uniF000"),
        '\u{F001}' => Some("fi"),
        '\u{F002}' => Some("fl"),
        '\u{F506}' => Some("one seventh"),
        '\u{F507}' => Some("two sevenths"),
        '\u{F508}' => Some("three sevenths"),
        '\u{F509}' => Some("four sevenths"),
        '\u{F50A}' => Some("five sevenths"),
        '\u{F50B}' => Some("six sevenths"),
        '\u{F50C}' => Some("one ninth"),
        '\u{F50D}' => Some("two ninths"),
        '\u{F50E}' => Some("four ninths"),
        '\u{F50F}' => Some("five ninths"),
        '\u{F510}' => Some("seven ninths"),
        '\u{F511}' => Some("eight ninths"),
        '\u{F800}' => Some("zero.alt"),
        '\u{F801}' => Some("one.alt"),
        '\u{F802}' => Some("two.alt"),
        '\u{F803}' => Some("three.alt"),
        '\u{F804}' => Some("four.alt"),
        '\u{F805}' => Some("five.alt"),
        '\u{F806}' => Some("six.alt"),
        '\u{F807}' => Some("seven.alt"),
        '\u{F808}' => Some("eight.alt"),
        '\u{F809}' => Some("nine.alt"),
        '\u{F80A}' => Some("zero.sups"),
        '\u{F80B}' => Some("one.sups"),
        '\u{F80C}' => Some("two.sups"),
        '\u{F80D}' => Some("three.sups"),
        '\u{F80E}' => Some("four.sups"),
        '\u{F80F}' => Some("five.sups"),
        '\u{F810}' => Some("six.sups"),
        '\u{F811}' => Some("seven.sups"),
        '\u{F812}' => Some("eight.sups"),
        '\u{F813}' => Some("nine.sups"),
        '\u{F814}' => Some("zero.sinf"),
        '\u{F815}' => Some("one.sinf"),
        '\u{F816}' => Some("two.sinf"),
        '\u{F817}' => Some("three.sinf"),
        '\u{F818}' => Some("four.sinf"),
        '\u{F819}' => Some("five.sinf"),
        '\u{F81A}' => Some("six.sinf"),
        '\u{F81B}' => Some("seven.sinf"),
        '\u{F81C}' => Some("eight.sinf"),
        '\u{F81D}' => Some("nine.sinf"),

        _ => None,
    }
}

pub fn icon_edit_button(ui: &mut egui::Ui, icon: &mut char, font_filter: &mut String) -> egui::Response {
    let popup_id = ui.auto_id_with("popup");
    let button_response = ui.button(RichText::new(icon.to_string()).size(24.0));
    if button_response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    if ui.memory(|mem| mem.is_popup_open(popup_id)) {
        let area_response = egui::Area::new(popup_id)
            .kind(egui::UiKind::Picker)
            .order(egui::Order::Foreground)
            .fixed_pos(button_response.rect.max)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.text_edit_singleline(font_filter);
                    let available_characters = available_characters(ui, egui::FontFamily::Proportional);
                    ui.allocate_ui(Vec2::new(ui.available_width(), 400.0), |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for chunk in available_characters.iter()
                                .filter(|(_, name)| name.contains(font_filter.as_str()))
                                .collect::<Vec<_>>().chunks(30) {
                                ui.horizontal(|ui| {
                                    for (chr,_name) in chunk {
                                        if ui.button(chr.to_string()).clicked() {
                                            *icon = *chr;
                                            ui.memory_mut(|mem| mem.close_popup());
                                        }
                                    };
                                });
                            }
                        });
                    });
                });
                if ui.button("close").clicked() {
                    ui.memory_mut(|mem| mem.close_popup());
                }
            })
            .response;

        if !button_response.clicked()
            && (ui.input(|i| i.key_pressed(egui::Key::Escape)) || area_response.clicked_elsewhere())
        {
            ui.memory_mut(|mem| mem.close_popup());
        }
    }

    button_response

    
    
}