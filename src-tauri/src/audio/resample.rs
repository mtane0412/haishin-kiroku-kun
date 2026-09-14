//! ダウンミックスと16kHzリサンプルを行うモジュールです。
//!
//! `cpal` から届く入力は任意のチャンネル数・サンプルレートのインターリーブ済み `f32`
//! バッファです。これを平均化によってモノラルへダウンミックスした上で、線形補間による
//! リサンプルで16kHzへ変換します。音声認識・VAD用途では帯域が広くないため、
//! FFTベースの高品質リサンプラーよりも実装が単純で低遅延な線形補間を採用しています。

/// エンジンへ配信する際の目標サンプルレート（Hz）です。
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// インターリーブされた複数チャンネルのサンプル列を、フレームごとの平均を取って
/// モノラルへダウンミックスします。`channels <= 1` の場合はそのまま返します。
pub fn downmix_to_mono(input: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return input.to_vec();
    }
    let channels = channels as usize;
    input
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// モノラルのサンプル列を `input_rate` から `output_rate` へ線形補間でリサンプルします。
/// `input` が空、または両者のレートが等しい場合は複製をそのまま返します。
pub fn resample_linear(input: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
    if input.is_empty() || input_rate == output_rate {
        return input.to_vec();
    }

    let ratio = output_rate as f64 / input_rate as f64;
    let output_len = ((input.len() as f64) * ratio).round() as usize;
    let last_index = input.len() - 1;

    (0..output_len)
        .map(|i| {
            let src_pos = i as f64 / ratio;
            let idx = (src_pos.floor() as usize).min(last_index);
            let frac = (src_pos - idx as f64) as f32;
            let s0 = input[idx];
            let s1 = input[(idx + 1).min(last_index)];
            s0 + (s1 - s0) * frac
        })
        .collect()
}

/// 入力バッファをダウンミックスした上で16kHzモノラルへリサンプルします。
pub fn downmix_and_resample_to_16k_mono(input: &[f32], channels: u16, input_rate: u32) -> Vec<f32> {
    let mono = downmix_to_mono(input, channels);
    resample_linear(&mono, input_rate, TARGET_SAMPLE_RATE)
}

/// サンプル列のRMS（二乗平均平方根）を計算します。
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// 指定した周波数・サンプルレート・長さの正弦波（振幅0.5）を生成します。
    fn sine_wave(frequency: f32, sample_rate: u32, num_frames: usize) -> Vec<f32> {
        (0..num_frames)
            .map(|i| 0.5 * (2.0 * PI * frequency * i as f32 / sample_rate as f32).sin())
            .collect()
    }

    #[test]
    fn 無音の48khzステレオを16khzモノラルへ変換するとサンプル数が3分の1になる() {
        let input_frames = 4_800; // 100ms分
        let input = vec![0.0f32; input_frames * 2]; // ステレオ・インターリーブ

        let output = downmix_and_resample_to_16k_mono(&input, 2, 48_000);

        assert_eq!(output.len(), input_frames / 3);
        assert!(output.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn 正弦波の48khzステレオを16khzモノラルへ変換すると振幅がほぼ維持される() {
        let input_frames = 4_800; // 100ms分
        let mono = sine_wave(440.0, 48_000, input_frames);
        // 左右同一波形のステレオ・インターリーブ入力を作る
        let mut input = Vec::with_capacity(input_frames * 2);
        for &s in &mono {
            input.push(s);
            input.push(s);
        }

        let output = downmix_and_resample_to_16k_mono(&input, 2, 48_000);
        let expected_len = input_frames / 3;

        assert_eq!(output.len(), expected_len);

        let input_rms = rms(&mono);
        let output_rms = rms(&output);
        assert!(
            (input_rms - output_rms).abs() < 0.05,
            "input_rms={input_rms}, output_rms={output_rms}"
        );
    }

    #[test]
    fn 入力レートと出力レートが同じ場合はそのまま返る() {
        let input = vec![0.1, 0.2, -0.3, 0.4];
        let output = resample_linear(&input, TARGET_SAMPLE_RATE, TARGET_SAMPLE_RATE);
        assert_eq!(output, input);
    }

    #[test]
    fn モノラル入力はダウンミックスされずそのまま返る() {
        let input = vec![0.1, -0.2, 0.3];
        let output = downmix_to_mono(&input, 1);
        assert_eq!(output, input);
    }
}
