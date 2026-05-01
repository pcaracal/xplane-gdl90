use std::{cell::RefCell, rc::Rc};

use egui::{RichText, TextStyle, Widget};
use egui_extras::{Column, TableBuilder};
#[allow(clippy::wildcard_imports)]
use uom::{
    fmt::DisplayStyle,
    si::{angle::*, length::*, velocity::*},
};
use xplm_egui::{egui_window::App, flight_loop::FlightLoop};

use crate::{
    data::Datarefs,
    fmt_uom,
    socket::{Socket, SocketState},
    state::State,
    util::DurationExt,
};

#[allow(unused)]
#[derive(Builder)]
pub struct EguiApp {
    state: State,
    datarefs: Rc<Datarefs>,
    flight_loop: Rc<RefCell<FlightLoop>>,
    socket: Socket,

    #[builder(skip)]
    input: Input,
}

#[derive(Default)]
struct Input {
    init: bool,

    target: String,
    target_error: Option<String>,

    interval: String,
    interval_error: Option<String>,
    interval_warning: Option<String>,
}

impl App for EguiApp {
    #[allow(clippy::too_many_lines)]
    fn ui(&mut self, ui: &mut egui::Ui, _: &xplm_egui::window::Window) {
        ui.set_zoom_factor(1.5);
        ui.style_mut().override_text_style = Some(TextStyle::Monospace);

        if !self.input.init {
            self.input.init = true;
            if let Some(target) = self.state.borrow().socket.target {
                self.input.target = target.to_string();
            }
            self.input.interval = self.state.borrow().socket.interval.human().to_string();
        }

        let error_color = ui.style().visuals.error_fg_color;
        let warning_color = ui.style().visuals.warn_fg_color;
        let target_color = self.input.target_error.is_some().then_some(error_color);
        let interval_color = if self.input.interval_error.is_some() {
            Some(error_color)
        } else if self.input.interval_warning.is_some() {
            Some(warning_color)
        } else {
            None
        };

        egui::CentralPanel::default_margins().show_inside(ui, |ui| {
            ui.vertical(|ui| {
                ui.label("Send interval");
                egui::TextEdit::singleline(&mut self.input.interval)
                    .hint_text("Example: 100ms")
                    .text_color_opt(interval_color)
                    .ui(ui);
                if let Some(error) = self
                    .input
                    .interval_error
                    .as_ref()
                    .or(self.input.interval_warning.as_ref())
                {
                    ui.colored_label(interval_color.unwrap_or(error_color), error);
                }

                ui.label("Target address");
                egui::TextEdit::singleline(&mut self.input.target)
                    .hint_text("Example: 192.168.1.1:4000")
                    .text_color_opt(target_color)
                    .ui(ui);
                if let Some(error) = &self.input.target_error {
                    ui.colored_label(error_color, error);
                }

                for (addr, info) in &self.state.borrow().socket.auto_targets {
                    ui.horizontal(|ui| {
                        ui.add_enabled(false, egui::TextEdit::singleline(&mut addr.to_string()));
                        ui.label(format!(
                            "{} ({:.1}s ago)",
                            info.app,
                            info.time.elapsed().as_secs_f32()
                        ));
                    });
                }

                ui.checkbox(&mut self.state.borrow_mut().heartbeat, "Heartbeat");
                ui.checkbox(&mut self.state.borrow_mut().ownship, "Ownship");
                ui.checkbox(&mut self.state.borrow_mut().ahrs, "AHRS");
                ui.checkbox(&mut self.state.borrow_mut().precise, "Precise Ownship");

                let data = self.datarefs.data();

                let names = [
                    "Latitude",
                    "Longitude",
                    "Altitude",
                    "Ground Speed",
                    "True Airspeed",
                    "Indicated Airspeed",
                    "Vertical Speed",
                    "Mag Heading",
                    "True Heading",
                ];

                let values = [
                    fmt_uom!(data.lat, degree, "{:.6}"),
                    fmt_uom!(data.lon, degree, "{:.6}"),
                    fmt_uom!(data.alt, foot, "{:.1}"),
                    fmt_uom!(data.gs, knot, "{:.1}"),
                    fmt_uom!(data.tas, knot, "{:.1}"),
                    fmt_uom!(data.ias, knot, "{:.1}"),
                    fmt_uom!(data.vs, foot_per_minute, "{:.1}"),
                    fmt_uom!(data.mag_hdg, degree, "{:.1}"),
                    fmt_uom!(data.true_hdg, degree, "{:.1}"),
                ];

                TableBuilder::new(ui)
                    .cell_layout(
                        egui::Layout::left_to_right(egui::Align::Center).with_cross_justify(true),
                    )
                    .column(Column::auto())
                    .column(Column::remainder())
                    .resizable(false)
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.heading("Name");
                        });
                        header.col(|ui| {
                            ui.heading("Value");
                        });
                    })
                    .body(|mut body| {
                        for (name, value) in names.into_iter().zip(values.iter()) {
                            body.row(20.0, |mut row| {
                                row.col(|ui| {
                                    egui::Label::new(RichText::new(name).monospace())
                                        .selectable(false)
                                        .ui(ui);
                                });
                                row.col(|ui| {
                                    egui::Label::new(RichText::new(value).monospace())
                                        .selectable(false)
                                        .ui(ui);
                                });
                            });
                        }
                    });
            });
        });

        if self.input.target.is_empty() {
            self.state.borrow_mut().socket.target = None;
            self.input.target_error = None;
        } else {
            match self.input.target.parse() {
                Ok(target) => {
                    self.state.borrow_mut().socket.target.replace(target);
                    self.input.target_error = None;
                }
                Err(why) => {
                    self.input.target_error = Some(why.to_string());
                }
            }
        }

        match humantime::parse_duration(&self.input.interval) {
            Ok(i) => {
                if i < SocketState::MINIMUM_INTERVAL {
                    self.input.interval_warning = Some(format!(
                        "Too low, using {}",
                        SocketState::MINIMUM_INTERVAL.human()
                    ));
                } else {
                    self.input.interval_warning = None;
                }
                self.input.interval_error = None;
                self.state.borrow_mut().socket.interval = i.max(SocketState::MINIMUM_INTERVAL);
            }
            Err(why) => {
                self.input.interval_error = Some(why.to_string());
            }
        }
    }
}
