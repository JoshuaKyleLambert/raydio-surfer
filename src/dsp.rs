use rodio::Source;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::num::{NonZeroU16, NonZeroU32};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const NUM_EQ_BANDS: usize = 10;
pub const EQ_FREQUENCIES: [f32; NUM_EQ_BANDS] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];
pub const EQ_BAND_LABELS: [&str; NUM_EQ_BANDS] = [
    "31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
];
pub const EQ_MIN_GAIN_DB: f32 = -12.0;
pub const EQ_MAX_GAIN_DB: f32 = 12.0;
pub const BALANCE_MIN: f32 = -1.0; // Full Left
pub const BALANCE_MAX: f32 = 1.0;  // Full Right

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioDspSettings {
    #[serde(default)]
    pub eq_enabled: bool,
    #[serde(default = "default_bands")]
    pub eq_bands: [f32; NUM_EQ_BANDS],
    #[serde(default)]
    pub balance: f32,
    #[serde(default)]
    pub eq_expanded: bool,
}

fn default_bands() -> [f32; NUM_EQ_BANDS] {
    [0.0; NUM_EQ_BANDS]
}

impl Default for AudioDspSettings {
    fn default() -> Self {
        Self {
            eq_enabled: false,
            eq_bands: [0.0; NUM_EQ_BANDS],
            balance: 0.0,
            eq_expanded: false,
        }
    }
}

/// Second-order IIR (Biquad) filter implementation based on Robert Bristow-Johnson's Audio EQ Cookbook.
#[derive(Debug, Clone, Copy)]
pub struct BiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Default for BiquadFilter {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
}

impl BiquadFilter {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset historical sample state
    pub fn reset_state(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }

    /// Configure as a peaking EQ filter
    pub fn set_peaking(&mut self, sample_rate: f32, freq: f32, gain_db: f32, q: f32) {
        if gain_db.abs() < 0.01 || sample_rate <= 0.0 {
            self.b0 = 1.0;
            self.b1 = 0.0;
            self.b2 = 0.0;
            self.a1 = 0.0;
            self.a2 = 0.0;
            return;
        }

        // Clamp center frequency to stay safely below Nyquist frequency
        let f0 = freq.clamp(10.0, (sample_rate * 0.49).max(10.0));
        let a = 10.0f32.powf(gain_db / 40.0);
        let omega0 = 2.0 * PI * f0 / sample_rate;
        let alpha = omega0.sin() / (2.0 * q.max(0.1));
        let cos_omega0 = omega0.cos();

        let a0 = 1.0 + alpha / a;
        let a0_inv = 1.0 / a0;

        self.b0 = (1.0 + alpha * a) * a0_inv;
        self.b1 = (-2.0 * cos_omega0) * a0_inv;
        self.b2 = (1.0 - alpha * a) * a0_inv;
        self.a1 = (-2.0 * cos_omega0) * a0_inv;
        self.a2 = (1.0 - alpha / a) * a0_inv;
    }

    /// Process a single audio sample through Direct Form I transposed difference equation
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;

        if y.is_finite() {
            y
        } else {
            self.reset_state();
            x
        }
    }
}

/// 10-Band Equalizer supporting multiple audio channels
#[derive(Debug, Clone)]
pub struct Equalizer {
    // 2 channels (Left, Right) x 10 frequency bands
    filters: [[BiquadFilter; NUM_EQ_BANDS]; 2],
    last_sample_rate: u32,
    last_gains: [f32; NUM_EQ_BANDS],
    last_enabled: bool,
}

impl Default for Equalizer {
    fn default() -> Self {
        Self {
            filters: [[BiquadFilter::default(); NUM_EQ_BANDS]; 2],
            last_sample_rate: 0,
            last_gains: [0.0; NUM_EQ_BANDS],
            last_enabled: false,
        }
    }
}

impl Equalizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, sample_rate: u32, enabled: bool, gains: &[f32; NUM_EQ_BANDS]) {
        if self.last_sample_rate == sample_rate
            && self.last_enabled == enabled
            && self.last_gains == *gains
        {
            return;
        }

        self.last_sample_rate = sample_rate;
        self.last_enabled = enabled;
        self.last_gains = *gains;

        if !enabled || sample_rate == 0 {
            return;
        }

        let sr = sample_rate as f32;
        // Standard Q factor for 1-octave 10-band graphic equalizer
        let q = 1.414;

        for (band_idx, &freq) in EQ_FREQUENCIES.iter().enumerate() {
            let gain = gains[band_idx].clamp(EQ_MIN_GAIN_DB, EQ_MAX_GAIN_DB);
            for ch in 0..2 {
                self.filters[ch][band_idx].set_peaking(sr, freq, gain, q);
            }
        }
    }

    #[inline]
    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        if !self.last_enabled {
            return sample;
        }

        let ch = channel.min(1);
        let mut out = sample;
        for band in 0..NUM_EQ_BANDS {
            if self.last_gains[band].abs() >= 0.01 {
                out = self.filters[ch][band].process(out);
            }
        }
        out
    }
}

/// Stereo Balance controller
#[derive(Debug, Clone, Copy, Default)]
pub struct BalanceControl;

impl BalanceControl {
    #[inline]
    pub fn process_sample(sample: f32, channel: usize, balance: f32) -> f32 {
        // If balance is at 0 (or close to center), balance processing is off/bypassed
        if balance.abs() < 0.005 {
            return sample;
        }

        let bal = balance.clamp(BALANCE_MIN, BALANCE_MAX);
        if channel == 0 {
            // Left Channel
            if bal > 0.0 {
                sample * (1.0 - bal).max(0.0)
            } else {
                sample
            }
        } else if channel == 1 {
            // Right Channel
            if bal < 0.0 {
                sample * (1.0 + bal).max(0.0)
            } else {
                sample
            }
        } else {
            sample
        }
    }
}

/// Rodio Source adapter wrapping any audio stream with real-time EQ and Balance processing
pub struct AudioDspSource<S> {
    source: S,
    dsp_settings: Arc<Mutex<AudioDspSettings>>,
    equalizer: Equalizer,
    current_channel: usize,
    sample_rate: NonZeroU32,
    channels: NonZeroU16,
    check_counter: usize,
    cached_balance: f32,
}

impl<S> AudioDspSource<S>
where
    S: Source<Item = f32>,
{
    pub fn new(source: S, dsp_settings: Arc<Mutex<AudioDspSettings>>) -> Self {
        let sample_rate = source.sample_rate();
        let channels = source.channels();
        let mut equalizer = Equalizer::new();
        let mut cached_balance = 0.0;

        if let Ok(settings) = dsp_settings.lock() {
            equalizer.update(sample_rate.into(), settings.eq_enabled, &settings.eq_bands);
            cached_balance = settings.balance;
        }

        Self {
            source,
            dsp_settings,
            equalizer,
            current_channel: 0,
            sample_rate,
            channels,
            check_counter: 0,
            cached_balance,
        }
    }

    #[inline]
    fn refresh_settings_if_needed(&mut self) {
        // Check settings periodically (every 128 samples) to avoid mutex locking overhead on every sample
        self.check_counter += 1;
        if self.check_counter >= 128 {
            self.check_counter = 0;
            if let Ok(settings) = self.dsp_settings.try_lock() {
                self.equalizer.update(self.sample_rate.into(), settings.eq_enabled, &settings.eq_bands);
                self.cached_balance = settings.balance;
            }
        }
    }
}

impl<S> Iterator for AudioDspSource<S>
where
    S: Source<Item = f32>,
{
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.source.next()?;
        self.refresh_settings_if_needed();

        let ch = self.current_channel;
        let num_ch = u16::from(self.channels) as usize;
        self.current_channel = if num_ch > 0 { (ch + 1) % num_ch } else { 0 };

        // 1. Equalizer Stage (bypassed if off or 0 dB)
        let eq_sample = self.equalizer.process_sample(sample, ch);

        // 2. Balance Stage (bypassed if 0)
        let final_sample = BalanceControl::process_sample(eq_sample, ch, self.cached_balance);

        Some(final_sample)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.source.size_hint()
    }
}

impl<S> Source for AudioDspSource<S>
where
    S: Source<Item = f32>,
{
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }

    #[inline]
    fn channels(&self) -> NonZeroU16 {
        self.channels
    }

    #[inline]
    fn sample_rate(&self) -> NonZeroU32 {
        self.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biquad_unity_passthrough_when_zero_gain() {
        let mut filter = BiquadFilter::new();
        filter.set_peaking(44100.0, 1000.0, 0.0, 1.414);

        let input = [0.1, -0.5, 0.8, -0.3, 0.0];
        for &sample in &input {
            let output = filter.process(sample);
            assert!((output - sample).abs() < 1e-4, "Sample {sample} changed to {output}");
        }
    }

    #[test]
    fn test_biquad_boost_and_cut() {
        let mut filter_boost = BiquadFilter::new();
        filter_boost.set_peaking(44100.0, 1000.0, 6.0, 1.414);

        let mut filter_cut = BiquadFilter::new();
        filter_cut.set_peaking(44100.0, 1000.0, -6.0, 1.414);

        // Sine wave at center frequency 1000 Hz
        let freq = 1000.0;
        let sr = 44100.0;
        let mut max_boost = 0.0f32;
        let mut max_cut = 0.0f32;
        let mut max_in = 0.0f32;

        for i in 0..500 {
            let t = i as f32 / sr;
            let sample = (2.0 * PI * freq * t).sin() * 0.5;
            let out_b = filter_boost.process(sample);
            let out_c = filter_cut.process(sample);

            if i > 200 { // steady state
                max_in = max_in.max(sample.abs());
                max_boost = max_boost.max(out_b.abs());
                max_cut = max_cut.max(out_c.abs());
            }
        }

        assert!(max_boost > max_in, "Boosted signal should have higher amplitude: {max_boost} > {max_in}");
        assert!(max_cut < max_in, "Cut signal should have lower amplitude: {max_cut} < {max_in}");
    }

    #[test]
    fn test_balance_control_laws() {
        // Center balance (0.0): unity gain on both channels
        assert_eq!(BalanceControl::process_sample(0.8, 0, 0.0), 0.8);
        assert_eq!(BalanceControl::process_sample(0.8, 1, 0.0), 0.8);

        // Full Left (-1.0): Left unity, Right muted (0.0)
        assert_eq!(BalanceControl::process_sample(0.8, 0, -1.0), 0.8);
        assert_eq!(BalanceControl::process_sample(0.8, 1, -1.0), 0.0);

        // Full Right (+1.0): Left muted (0.0), Right unity
        assert_eq!(BalanceControl::process_sample(0.8, 0, 1.0), 0.0);
        assert_eq!(BalanceControl::process_sample(0.8, 1, 1.0), 0.8);

        // Partial Left (-0.5): Left unity, Right at 50%
        assert_eq!(BalanceControl::process_sample(1.0, 0, -0.5), 1.0);
        assert_eq!(BalanceControl::process_sample(1.0, 1, -0.5), 0.5);
    }

    #[test]
    fn test_equalizer_bypass_when_disabled() {
        let mut eq = Equalizer::new();
        let mut gains = [0.0; NUM_EQ_BANDS];
        gains[3] = 10.0; // Boost band 3
        eq.update(44100, false, &gains); // Disabled

        let out = eq.process_sample(0.75, 0);
        assert_eq!(out, 0.75); // Bypassed
    }

    #[test]
    fn test_all_10_eq_bands_frequencies_and_labels() {
        assert_eq!(EQ_FREQUENCIES.len(), 10);
        assert_eq!(EQ_BAND_LABELS.len(), 10);
        assert_eq!(EQ_FREQUENCIES[0], 31.0);
        assert_eq!(EQ_FREQUENCIES[9], 16000.0);

        let mut eq = Equalizer::new();
        let gains = [3.0; NUM_EQ_BANDS];
        eq.update(48000, true, &gains);

        let sample = 0.5f32;
        for ch in 0..2 {
            let out = eq.process_sample(sample, ch);
            assert!(out.is_finite());
        }
    }

    #[test]
    fn test_audio_dsp_source_pipeline() {
        // Mock source generating 100 constant samples
        struct MockSource {
            samples: Vec<f32>,
            pos: usize,
            channels: NonZeroU16,
            sample_rate: NonZeroU32,
        }

        impl Iterator for MockSource {
            type Item = f32;
            fn next(&mut self) -> Option<Self::Item> {
                if self.pos < self.samples.len() {
                    let s = self.samples[self.pos];
                    self.pos += 1;
                    Some(s)
                } else {
                    None
                }
            }
        }

        impl Source for MockSource {
            fn current_span_len(&self) -> Option<usize> {
                Some(self.samples.len() - self.pos)
            }
            fn channels(&self) -> NonZeroU16 {
                self.channels
            }
            fn sample_rate(&self) -> NonZeroU32 {
                self.sample_rate
            }
            fn total_duration(&self) -> Option<Duration> {
                None
            }
        }

        let settings = Arc::new(Mutex::new(AudioDspSettings {
            eq_enabled: true,
            eq_bands: [0.0; 10],
            balance: -1.0, // Full Left (Right muted)
            eq_expanded: true,
        }));

        let mock = MockSource {
            samples: vec![1.0; 10], // Interleaved [L, R, L, R, ...]
            pos: 0,
            channels: NonZeroU16::new(2).unwrap(),
            sample_rate: NonZeroU32::new(44100).unwrap(),
        };

        let dsp_src = AudioDspSource::new(mock, settings);
        let output: Vec<f32> = dsp_src.collect();

        assert_eq!(output.len(), 10);
        // Left channel samples should be 1.0, Right channel samples should be 0.0
        for (i, &sample) in output.iter().enumerate() {
            if i % 2 == 0 {
                // Left
                assert!((sample - 1.0).abs() < 1e-3, "Left sample should be 1.0, got {sample}");
            } else {
                // Right
                assert!((sample - 0.0).abs() < 1e-3, "Right sample should be 0.0, got {sample}");
            }
        }
    }
}
