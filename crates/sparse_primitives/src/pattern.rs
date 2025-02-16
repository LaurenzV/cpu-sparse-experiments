use crate::Pixmap;
use std::sync::Arc;

#[derive(Debug)]
struct Repr {
    alpha: f32,
}

#[derive(Debug, Clone)]
pub struct Pattern(Arc<Repr>);

impl Pattern {
    pub fn new(_: Pixmap, alpha: f32, _: FilterQuality) -> Self {
        Self(Arc::new(Repr { alpha }))
    }

    pub fn alpha(&self) -> f32 {
        self.0.alpha
    }
}

#[derive(Copy, Clone, Debug)]
pub enum FilterQuality {
    Nearest,
}
