use crate::ExecutionMode;

pub struct Dispatcher<'a, T> {
    pub scalar: Box<dyn Fn(T) + 'a>,
    pub execution_mode: ExecutionMode,
    #[cfg(all(target_arch = "aarch64", feature = "simd"))]
    pub neon: Box<dyn Fn(T) + 'a>,
}

impl<'a, T> Dispatcher<'a, T> {
    pub fn dispatch(&self, params: T) {
        match self.execution_mode {
            ExecutionMode::Scalar => {
                return (self.scalar)(params);
            }
            #[cfg(feature = "simd")]
            ExecutionMode::Auto => {
                #[cfg(target_arch = "aarch64")]
                if std::arch::is_aarch64_feature_detected!("neon") {
                    return (self.neon)(params);
                }

                // Fallback.
                return (self.scalar)(params);
            }
            #[cfg(all(target_arch = "aarch64", feature = "simd"))]
            ExecutionMode::Neon => {
                if std::arch::is_aarch64_feature_detected!("neon") {
                    return (self.neon)(params);
                }

                panic!(
                    "attempted to force execution mode NEON, but CPU doesn't support NEON instructions"
                );
            }
        }
    }
}
