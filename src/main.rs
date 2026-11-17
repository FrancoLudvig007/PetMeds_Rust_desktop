// src/main.rs
#[macro_use]
pub mod pyrsutils;
pub mod app;
pub mod controllers;
pub mod dao;
pub mod db;
pub mod models;
pub mod views;

use app::PetMedsApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 740.0])
            .with_min_inner_size([420.0, 600.0])
            .with_title("🐾 PetMeds Desktop — Gestão Colaborativa de Medicação Pet"),
        ..Default::default()
    };

    eframe::run_native(
        "PetMeds Desktop",
        options,
        Box::new(|cc| Ok(Box::new(PetMedsApp::new(cc)))),
    )
}
