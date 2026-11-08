//! The time now, for the displays: std's `Instant`, and in the browser
//! (where std has no clock to read) one on the page's clock, which the page
//! hands the module as `env.now_ms` (`performance.now()`).

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;

#[cfg(target_arch = "wasm32")]
pub use web::Instant;

#[cfg(target_arch = "wasm32")]
mod web {
    use std::time::Duration;

    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        fn now_ms() -> f64;
    }

    /// A moment, as milliseconds on the page's clock.
    #[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
    pub struct Instant(f64);

    impl Instant {
        pub fn now() -> Instant {
            // SAFETY: the page's function, taking nothing
            Instant(unsafe { now_ms() })
        }

        pub fn duration_since(&self, earlier: Instant) -> Duration {
            Duration::from_secs_f64(((self.0 - earlier.0) / 1000.0).max(0.0))
        }

        pub fn elapsed(&self) -> Duration {
            Instant::now().duration_since(*self)
        }
    }

    impl std::ops::Add<Duration> for Instant {
        type Output = Instant;
        fn add(self, d: Duration) -> Instant {
            Instant(self.0 + d.as_secs_f64() * 1000.0)
        }
    }

    impl std::ops::Sub for Instant {
        type Output = Duration;
        fn sub(self, earlier: Instant) -> Duration {
            self.duration_since(earlier)
        }
    }
}
