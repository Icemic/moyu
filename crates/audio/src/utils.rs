/// Converts a linear volume value (0.0–1.0) to `Decibels` so that the amplitude
/// scales linearly with the input value (i.e. 0.5 → -6 dB → amplitude 0.5).
pub(crate) fn linear_volume(v: f64) -> kira::Decibels {
    if v <= 0.0 {
        kira::Decibels::SILENCE
    } else {
        kira::Decibels((20.0 * v.log10()) as f32)
    }
}
