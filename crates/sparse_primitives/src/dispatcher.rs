pub struct Dispatcher<'a, T> {
    pub scalar: Box<dyn Fn(T) + 'a>,
    pub use_simd: bool,
    #[cfg(all(target_arch = "aarch64", feature = "simd"))]
    pub neon: Box<dyn Fn(T) + 'a>,
}

impl<'a, T> Dispatcher<'a, T> {
    pub fn new<F: Fn(T) + Clone + 'a>(
        scalar: F,
        use_simd: bool,
    ) -> Self {
        Self {
            scalar: Box::new(scalar.clone()),
            use_simd,
            #[cfg(all(target_arch = "aarch64", feature = "simd"))]
            neon: Box::new(scalar.clone()),
        }
    }

    #[cfg(all(target_arch = "aarch64", feature = "simd"))]
    pub fn with_neon<F: Fn(T) + 'a>(mut self, neon: F) -> Self {
        self.neon = Box::new(neon);
        self
    }

    pub fn dispatch(&self, params: T) {
        #[cfg(feature = "simd")]
        if self.use_simd {
            #[cfg(target_arch = "aarch64")]
            if std::arch::is_aarch64_feature_detected!("neon") {
                return (self.neon)(params);
            }
        }

        (self.scalar)(params);
    }
}
