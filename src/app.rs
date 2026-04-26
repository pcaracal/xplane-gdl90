use std::{cell::RefCell, net::SocketAddr, rc::Rc, str::FromStr};

use egui::{RichText, Widget};
use egui_extras::{Column, TableBuilder};
#[allow(clippy::wildcard_imports)]
use uom::{
    fmt::DisplayStyle,
    si::{angle::*, length::*, velocity::*},
};
use xplm_egui::{egui_window::App, flight_loop::FlightLoop};

use crate::{data::Datarefs, fmt_uom, socket::Socket, state::State, util::DurationExt};

#[allow(unused)]
#[derive(Builder)]
pub struct EguiApp {
    state: Rc<State>,
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
        if !self.input.init {
            self.input.init = true;
            if let Some(target) = self.state.target.get() {
                self.input.target = target.to_string();
            }
            self.input.interval = self.state.interval.get().human().to_string();
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
        let mut old_target = self.state.target.get();
        let mut old_interval = self.state.interval.get();

        egui::CentralPanel::default_margins().show_inside(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal_wrapped(|ui| {
                    if egui::TextEdit::singleline(&mut self.input.target)
                        .hint_text("192.168.1.1:4000")
                        .text_color_opt(target_color)
                        .ui(ui)
                        .changed()
                    {
                        match SocketAddr::from_str(&self.input.target) {
                            Ok(addr) => {
                                old_target = self.state.target.replace(Some(addr));
                                self.input.target_error = None;
                            }
                            Err(why) => {
                                old_target = self.state.target.replace(None);
                                self.input.target_error = Some(why.to_string());
                            }
                        }
                    }

                    if let Some(error) = &self.input.target_error {
                        ui.label(RichText::new(error).color(error_color));
                    }
                });

                ui.horizontal_wrapped(|ui| {
                    if egui::TextEdit::singleline(&mut self.input.interval)
                        .hint_text("1s")
                        .text_color_opt(interval_color)
                        .ui(ui)
                        .changed()
                    {
                        match humantime::parse_duration(&self.input.interval) {
                            Ok(dur) => {
                                if dur < State::MINIMUM_INTERVAL {
                                    self.input.interval_warning = Some(format!(
                                        "Too low, using {}",
                                        State::MINIMUM_INTERVAL.human()
                                    ));
                                    old_interval =
                                        self.state.interval.replace(State::MINIMUM_INTERVAL);
                                    self.input.interval_error = None;
                                } else {
                                    old_interval = self.state.interval.replace(dur);
                                    self.input.interval_warning = None;
                                    self.input.interval_error = None;
                                }
                            }
                            Err(why) => {
                                old_interval = self.state.interval.replace(State::DEFAULT_INTERVAL);
                                self.input.interval_error = Some(format!(
                                    "Error: {why}, using {}",
                                    State::MINIMUM_INTERVAL.human()
                                ));
                            }
                        }
                    }

                    if let Some(warning) = &self.input.interval_warning {
                        ui.label(RichText::new(warning).color(ui.style().visuals.warn_fg_color));
                    }

                    if let Some(error) = &self.input.interval_error {
                        ui.label(RichText::new(error).color(ui.style().visuals.error_fg_color));
                    }
                });

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

        let t = old_target != self.state.target.get();
        let i = old_interval != self.state.interval.get();
        if t || i {
            if t {
                self.socket.set_target(self.state.target.get());
            }

            if i {
                self.flight_loop
                    .borrow_mut()
                    .schedule_after(self.state.interval.get());
            }

            self.state.save();
        }
    }
}
