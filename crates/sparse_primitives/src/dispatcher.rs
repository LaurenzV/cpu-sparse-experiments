pub struct Dispatcher<'a, T> {
    pub scalar: Box<dyn Fn(T) + 'a>,
    #[cfg(feature = "simd")]
    pub use_simd: bool,
    #[cfg(all(target_arch = "aarch64", feature = "simd"))]
    pub neon: Box<dyn Fn(T) + 'a>,
}

impl<'a, T> Dispatcher<'a, T> {
    pub fn dispatch(&self, params: T) {
        if option_env!("FORCE_NEON").is_some() {
            #[cfg(all(target_arch = "aarch64", feature = "simd"))]
            if std::arch::is_aarch64_feature_detected!("neon") {
                return (self.neon)(params);
            }

            panic!(
                "attempted to force execution of neon SIMD kernel, \
            but CPU doesn't support NEON instructions or the SIMD feature wasn't enabled"
            );
        }

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
