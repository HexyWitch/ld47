use euclid::default::Size2D;

pub const TICK_DT: f32 = 1. / 60.;

pub const SCREEN_SIZE: Size2D<u32> = euclid::Size2D {
    width: 1280,
    height: 720,
    _unit: std::marker::PhantomData::<euclid::UnknownUnit>,
};
pub const ZOOM_LEVEL: f32 = 2.;
