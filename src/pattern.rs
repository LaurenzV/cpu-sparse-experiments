use crate::Pixmap;
use std::sync::Arc;

#[derive(Debug)]
struct Repr {
    pixmap: Pixmap,
    alpha: f32,
    filter_quality: FilterQuality,
}

#[derive(Debug, Clone)]
pub struct Pattern(Arc<Repr>);

impl Pattern {
    pub fn new(pixmap: Pixmap, alpha: f32, filter_quality: FilterQuality) -> Self {
        Self(Arc::new(Repr {
            pixmap,
            alpha,
            filter_quality,
        }))
    }

    pub fn alpha(&self) -> f32 {
        self.0.alpha
    }
}

#[derive(Copy, Clone, Debug)]
pub enum FilterQuality {
    Nearest,
}
