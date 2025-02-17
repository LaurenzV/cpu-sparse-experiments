/// A macro for dispatching to kernels. Make sure that you pass the correct function
/// to each argument, as this macro will call into it using unsafe code, assuming that for example
/// the function in `neon` only uses `neon` instructions!
#[macro_export]
macro_rules! dispatch {
    (
        scalar: $scalar:expr,
        neon: $neon:expr,
        execution_mode: $execution_mode:expr
    ) => {
        match $execution_mode {
            ExecutionMode::Scalar => {
                return $scalar;
            }
            #[cfg(feature = "simd")]
            ExecutionMode::Auto => {
                #[cfg(target_arch = "aarch64")]
                if std::arch::is_aarch64_feature_detected!("neon") {
                    return unsafe { $neon };
                }

                // Fallback.
                return $scalar;
            }
            #[cfg(all(target_arch = "aarch64", feature = "simd"))]
            ExecutionMode::Neon => {
                if std::arch::is_aarch64_feature_detected!("neon") {
                    return unsafe { $neon };
                }

                panic!(
                    "attempted to force execution mode NEON, but CPU doesn't support NEON instructions"
                );
            }
        }
    };
}
