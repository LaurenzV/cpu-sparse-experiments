use peniko::color::{AlphaColor, Srgb};

#[derive(Debug, Clone)]
pub enum Paint {
    Solid(AlphaColor<Srgb>),
}

impl From<AlphaColor<Srgb>> for Paint {
    fn from(value: AlphaColor<Srgb>) -> Self {
        Paint::Solid(value)
    }
}

impl Paint {
    pub fn alpha(&self) -> f32 {
        match self {
            Paint::Solid(s) => s.components[3],
        }
    }
}
