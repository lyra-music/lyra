use std::num::NonZeroI64;

const TWELFTH_ROOT_OF_TWO: f64 = 1.059_463_094_359_295_3;

#[derive(Clone)]
pub struct Pitch {
    multiplier: f64,
    half_tone_shifts: i64,
}

impl Pitch {
    const DEFAULT_MULTIPLIER: f64 = 1.;

    pub(super) const fn new() -> Self {
        Self {
            multiplier: Self::DEFAULT_MULTIPLIER,
            half_tone_shifts: 0,
        }
    }

    pub fn set(&mut self, multiplier: f64) {
        self.multiplier = multiplier;
        self.half_tone_shifts = 0;
    }

    #[inline]
    pub fn reset(&mut self) {
        self.set(Self::DEFAULT_MULTIPLIER);
    }

    #[inline]
    pub fn get(&self) -> f64 {
        #[allow(clippy::cast_possible_truncation)]
        let half_ton_shifts_i32 = self.half_tone_shifts as i32;
        self.multiplier * TWELFTH_ROOT_OF_TWO.powi(half_ton_shifts_i32)
    }

    #[inline]
    pub fn checked_get(&self) -> Option<f64> {
        const ERR_MARGIN: f64 = f64::EPSILON;

        let value = self.get();
        ((value - Self::DEFAULT_MULTIPLIER).abs() > ERR_MARGIN).then_some(value)
    }

    pub fn shift(&mut self, half_tones: NonZeroI64) {
        self.half_tone_shifts += half_tones.get();
    }

    #[inline]
    pub fn clone_before_and_after_shifted(&mut self, half_tones: NonZeroI64) -> (Self, Self) {
        let old = self.clone();
        self.shift(half_tones);
        (old, self.clone())
    }
}

impl std::fmt::Display for Pitch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}× {:+}♯ (≈{:.3}×)",
            self.multiplier,
            self.half_tone_shifts,
            self.get()
        )
    }
}
