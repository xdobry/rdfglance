#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use domain::graph_styles::{EdgeStyle, NodeStyle};

use crate::{
    domain::{NodeChangeContext, RdfData},
    uistate::{DisplayType, SystemMessage},
};

pub mod domain;
pub mod graph_algorithms;
pub mod integration;
pub mod layoutalg;
pub mod support;
pub mod ui;
pub mod uistate;

pub use domain::IriIndex;
pub use uistate::app::RdfGlanceApp;
