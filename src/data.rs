use xplm_egui::data::typed::TypedDataRead;

xplm_egui::uom_typed_dataref!(
    name: angle,
    type: uom::si::angle::Angle,
    unit: uom::si::angle::degree,
    range: -360.0..=360.0
);

xplm_egui::uom_typed_dataref!(
    name: velocity_mps,
    type: uom::si::velocity::Velocity,
    unit: uom::si::velocity::meter_per_second,
    range: f64::MIN..f64::MAX
);

xplm_egui::uom_typed_dataref!(
    name: velocity_kt,
    type: uom::si::velocity::Velocity,
    unit: uom::si::velocity::knot,
    range: f64::MIN..f64::MAX
);

xplm_egui::uom_typed_dataref!(
    name: length,
    type: uom::si::length::Length,
    unit: uom::si::length::meter,
    range: f64::MIN..f64::MAX
);

xplm_egui::uom_typed_dataref!(
    name: pressure,
    type: uom::si::pressure::Pressure,
    unit: uom::si::pressure::pascal,
    range: f64::MIN..f64::MAX
);

#[derive(Default, Debug, Clone, Copy)]
pub struct Data {
    pub lat: uom::si::f64::Angle,
    pub lon: uom::si::f64::Angle,
    pub alt: uom::si::f64::Length,
    pub gs: uom::si::f64::Velocity,
    pub tas: uom::si::f64::Velocity,
    pub ias: uom::si::f64::Velocity,
    pub vs: uom::si::f64::Velocity,
    pub mag_hdg: uom::si::f64::Angle,
    pub true_hdg: uom::si::f64::Angle,
}

pub struct Datarefs {
    lat: angle::DataRef<f64>,
    lon: angle::DataRef<f64>,
    alt: length::DataRef<f64>,
    gs: velocity_mps::DataRef<f32>,
    tas: velocity_mps::DataRef<f32>,
    ias: velocity_kt::DataRef<f32>,
    vs: velocity_mps::DataRef<f32>,
    mag_hdg: angle::DataRef<f32>,
    true_hdg: angle::DataRef<f32>,
}

macro_rules! get_uom {
    ($in:expr, $u:ty, $ut:ty) => {
        pastey::paste! {
            uom::si::f64::$ut::new::<uom::si::[<$ut:lower>]::$u>(f64::from($in.get().unwrap_or_default().get::<uom::si::[<$ut:lower>]::$u>()))
        }
    };

    ($in:expr) => {
        $in.get().unwrap_or_default()
    }
}

impl Datarefs {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            lat: angle::DataRef::find("sim/flightmodel/position/latitude")?,
            lon: angle::DataRef::find("sim/flightmodel/position/longitude")?,
            alt: length::DataRef::find("sim/flightmodel/position/elevation")?,
            gs: velocity_mps::DataRef::find("sim/flightmodel/position/groundspeed")?,
            tas: velocity_mps::DataRef::find("sim/flightmodel/position/true_airspeed")?,
            ias: velocity_kt::DataRef::find("sim/flightmodel/position/indicated_airspeed")?,
            vs: velocity_mps::DataRef::find("sim/flightmodel/position/vh_ind")?,
            mag_hdg: angle::DataRef::find("sim/flightmodel/position/mag_psi")?,
            true_hdg: angle::DataRef::find("sim/flightmodel/position/true_psi")?,
        })
    }

    #[must_use]
    pub fn data(&self) -> Data {
        Data {
            lat: get_uom!(self.lat),
            lon: get_uom!(self.lon),
            alt: get_uom!(self.alt),
            gs: get_uom!(self.gs, knot, Velocity),
            tas: get_uom!(self.tas, knot, Velocity),
            ias: get_uom!(self.ias, knot, Velocity),
            vs: get_uom!(self.vs, foot_per_minute, Velocity),
            mag_hdg: get_uom!(self.mag_hdg, degree, Angle),
            true_hdg: get_uom!(self.true_hdg, degree, Angle),
        }
    }
}
