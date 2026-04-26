use std::time::Instant;

use anyhow::bail;
use bitflags::bitflags;
use egui::Ui;
use egui_extras::{Column, TableBuilder};
use xplm_egui::{
    data::{ArrayRead, DataRead, borrowed::DataRef},
    sys::{XPLMDataRef, XPLMDataRefInfo_t, XPLMGetDataRefInfo, XPLMGetDataRefsByIndex},
};

use crate::{state::State, util::DurationExt};

#[derive(Default)]
pub struct DatarefViewer {
    datarefs: Vec<Wrapped>,
}

struct Wrapped {
    name: String,
    dataref: Typed,
    updated: Instant,
    value: String,
}

impl Wrapped {
    fn update(&mut self) {
        if self.updated.elapsed() < std::time::Duration::from_secs(1) {
            return;
        }

        self.value = match &self.dataref {
            Typed::Int(dataref) => dataref.get().to_string(),
            Typed::Float(dataref) => dataref.get().to_string(),
            Typed::Double(dataref) => dataref.get().to_string(),
            Typed::FloatArray(dataref) => {
                let values = dataref.as_vec();
                if values.len() <= 10 {
                    format!("{values:?}")
                } else {
                    format!(
                        "[{}, ... ({} more)]",
                        values
                            .iter()
                            .take(10)
                            .map(std::string::ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", "),
                        values.len() - 10
                    )
                }
            }
            Typed::IntArray(dataref) => {
                let values = dataref.as_vec();
                if values.len() <= 10 {
                    format!("{values:?}")
                } else {
                    format!(
                        "[{}, ... ({} more)]",
                        values
                            .iter()
                            .take(10)
                            .map(std::string::ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", "),
                        values.len() - 10
                    )
                }
            }
        };
        self.updated = Instant::now();
    }
}

enum Typed {
    Int(DataRef<i32>),
    Float(DataRef<f32>),
    Double(DataRef<f64>),
    FloatArray(DataRef<[f32]>),
    IntArray(DataRef<[i32]>),
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DatarefType: i32 {
        /// "0"	Data of a type the current XPLM doesn't do.
        const UNKNOWN = 0;
        /// "1"	A single 4-byte integer, native endian.
        const INT = 1;
        /// "2"	A single 4-byte float, native endian.
        const FLOAT = 2;
        /// "4"	A single 8-byte double, native endian.
        const DOUBLE = 4;
        /// "8"	An array of 4-byte floats, native endian.
        const FLOAT_ARRAY = 8;
        /// "16"	An array of 4-byte integers, native endian.
        const INT_ARRAY = 16;
        /// "32"	A variable block of data.
        const DATA = 32;
    }
}

fn find_dref(dt: DatarefType, name: &str) -> anyhow::Result<Typed> {
    Ok(match dt {
        _ if dt.contains(DatarefType::DOUBLE) => Typed::Double(DataRef::find(name)?),
        _ if dt.contains(DatarefType::FLOAT) => Typed::Float(DataRef::find(name)?),
        _ if dt.contains(DatarefType::INT) => Typed::Int(DataRef::find(name)?),
        _ if dt.contains(DatarefType::FLOAT_ARRAY) => Typed::FloatArray(DataRef::find(name)?),
        _ if dt.contains(DatarefType::INT_ARRAY) => Typed::IntArray(DataRef::find(name)?),
        _ => {
            bail!("Unsupported data type: {dt:?}")
        }
    })
}

impl DatarefViewer {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap,
        clippy::cast_possible_truncation
    )]
    pub fn init(&mut self) {
        let start = Instant::now();

        self.datarefs.clear();

        unsafe {
            let count = xplm_egui::sys::XPLMCountDataRefs();
            debug!("Dataref count: {count}");

            let mut datarefs: Vec<XPLMDataRef> = vec![std::ptr::null_mut(); count as usize];

            XPLMGetDataRefsByIndex(0, count, datarefs.as_mut_ptr());
            for dataref in &datarefs {
                if dataref.is_null() {
                    continue;
                }

                let mut info: XPLMDataRefInfo_t = std::mem::zeroed();
                info.structSize = std::mem::size_of::<XPLMDataRefInfo_t>() as i32;
                XPLMGetDataRefInfo(*dataref, &raw mut info);
                let name = std::ffi::CStr::from_ptr(info.name).to_string_lossy();
                let Some(data_type) = DatarefType::from_bits(info.type_) else {
                    continue;
                };

                if let Ok(dataref) = find_dref(data_type, &name) {
                    let mut w = Wrapped {
                        name: name.to_string(),
                        dataref,
                        updated: Instant::now(),
                        value: String::new(),
                    };
                    w.update();
                    self.datarefs.push(w);
                }
            }
        }

        info!(
            "Found {} datarefs in {}",
            self.datarefs.len(),
            start.elapsed().human()
        );
    }

    pub fn ui(&mut self, ui: &mut Ui, _: &State) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
            ui.heading(format!("{} Datarefs", self.datarefs.len()));

            if ui.button("init").clicked() {
                self.init();
            }
        });

        TableBuilder::new(ui)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto()) // name
            .column(Column::auto()) // type
            .column(Column::remainder()) // value
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.heading("Name");
                });
                header.col(|ui| {
                    ui.heading("Type");
                });
                header.col(|ui| {
                    ui.heading("Value");
                });
            })
            .body(|body| {
                body.rows(18.0, self.datarefs.len(), |mut row| {
                    let index = row.index();
                    let wrapped = &mut self.datarefs[index];
                    row.col(|ui| {
                        ui.monospace(&wrapped.name);
                    });
                    row.col(|ui| {
                        let type_name = match &wrapped.dataref {
                            Typed::Int(_) => "i32",
                            Typed::Float(_) => "f32",
                            Typed::Double(_) => "f64",
                            Typed::FloatArray(_) => "[f32]",
                            Typed::IntArray(_) => "[i32]",
                        };
                        ui.monospace(type_name);
                    });
                    row.col(|ui| {
                        wrapped.update();
                        ui.monospace(&wrapped.value);
                    });
                });
            });
    }
}
