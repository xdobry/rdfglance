#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]


use domain::graph_styles::{EdgeStyle, NodeStyle};


use crate::{
    domain::{
        NodeChangeContext, RdfData,
    }, 
    uistate::{DisplayType, SystemMessage}
};

pub mod graph_algorithms;
pub mod layoutalg;
pub mod domain;
pub mod ui;
pub mod uistate;
pub mod integration;
pub mod support;

pub use domain::IriIndex;
pub use uistate::app::RdfGlanceApp;


