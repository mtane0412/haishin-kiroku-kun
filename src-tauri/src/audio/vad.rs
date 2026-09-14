//! エネルギーベースのVAD（発話区間検出）セグメンタです。
//!
//! 16kHzモノラルの音声ストリームを一定時間幅（フレーム）ごとのRMSエネルギーで判定し、
//! 無音→発話→無音の区間を1つの [`Segment`] として切り出します。発話が
//! [`VadConfig::max_segment_ms`] を超えて継続する場合は、無音を待たずにその時点で
//! 強制的にセグメントを区切ります。

use super::resample::rms;

/// VADの挙動を決めるパラメータです。
///
/// フェーズ3（文字起こしエンジン連携）で `AudioFrame` ストリームに対して
/// `VadSegmenter` を配線する予定のため、現時点ではテストからのみ使用しており
/// 未使用警告を抑制しています。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct VadConfig {
    /// 入力サンプルレート（Hz）。
    pub sample_rate: u32,
    /// エネルギー判定を行うフレーム長（ミリ秒）。
    pub frame_ms: u32,
    /// 発話とみなすRMSエネルギーの閾値。
    pub energy_threshold: f32,
    /// この長さ以上無音が続いたら発話区間を終了します（ミリ秒）。
    pub min_silence_ms: u32,
    /// 発話区間の最大長です（ミリ秒）。超えた場合は無音を待たずに区切ります。
    pub max_segment_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            sample_rate: super::resample::TARGET_SAMPLE_RATE,
            frame_ms: 20,
            energy_threshold: 0.02,
            min_silence_ms: 500,
            max_segment_ms: 15_000,
        }
    }
}

/// 切り出された1つの発話区間です。
///
/// フェーズ3で使用予定のため、現時点ではテストからのみ使用しており未使用警告を抑制しています。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    /// 区間開始時刻（ストリーム先頭からの経過ミリ秒）。
    pub start_ms: i64,
    /// 区間終了時刻（ストリーム先頭からの経過ミリ秒）。
    pub end_ms: i64,
    /// 区間に含まれるサンプル列。
    pub samples: Vec<f32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Silence,
    Speech,
}

/// ストリーミングでサンプルを受け取り、発話区間を切り出すステートマシンです。
///
/// フェーズ3で使用予定のため、現時点ではテストからのみ使用しており未使用警告を抑制しています。
#[allow(dead_code)]
pub struct VadSegmenter {
    config: VadConfig,
    frame_len: usize,
    state: State,
    /// `frame_len` に満たない端数サンプルを次回の `push` まで持ち越すバッファです。
    pending: Vec<f32>,
    segment: Vec<f32>,
    segment_start_ms: i64,
    silence_run_ms: u32,
    /// ストリーム先頭からの経過ミリ秒（次に処理するフレームの開始時刻）。
    elapsed_ms: i64,
}

#[allow(dead_code)]
impl VadSegmenter {
    /// 指定した設定で `VadSegmenter` を作成します。
    pub fn new(config: VadConfig) -> Self {
        let frame_len = ((config.sample_rate as u64 * config.frame_ms as u64) / 1000) as usize;
        Self {
            config,
            frame_len: frame_len.max(1),
            state: State::Silence,
            pending: Vec::new(),
            segment: Vec::new(),
            segment_start_ms: 0,
            silence_run_ms: 0,
            elapsed_ms: 0,
        }
    }

    /// 音声サンプルを追加します。この呼び出しで確定した発話区間があれば返します。
    pub fn push(&mut self, samples: &[f32]) -> Vec<Segment> {
        self.pending.extend_from_slice(samples);
        let mut finished = Vec::new();

        while self.pending.len() >= self.frame_len {
            let frame: Vec<f32> = self.pending.drain(..self.frame_len).collect();
            let frame_start_ms = self.elapsed_ms;
            self.elapsed_ms += self.config.frame_ms as i64;

            let is_speech = rms(&frame) >= self.config.energy_threshold;

            match self.state {
                State::Silence => {
                    if is_speech {
                        self.state = State::Speech;
                        self.segment_start_ms = frame_start_ms;
                        self.segment.clear();
                        self.segment.extend_from_slice(&frame);
                        self.silence_run_ms = 0;
                    }
                }
                State::Speech => {
                    self.segment.extend_from_slice(&frame);
                    if is_speech {
                        self.silence_run_ms = 0;
                    } else {
                        self.silence_run_ms += self.config.frame_ms;
                        if self.silence_run_ms >= self.config.min_silence_ms {
                            finished.push(self.close_segment(self.elapsed_ms));
                            continue;
                        }
                    }

                    let duration_ms = self.elapsed_ms - self.segment_start_ms;
                    if duration_ms >= self.config.max_segment_ms as i64 {
                        finished.push(self.close_segment(self.elapsed_ms));
                    }
                }
            }
        }

        finished
    }

    /// 現在の発話区間を確定させ、状態を無音へ戻します。
    fn close_segment(&mut self, end_ms: i64) -> Segment {
        let segment = Segment {
            start_ms: self.segment_start_ms,
            end_ms,
            samples: std::mem::take(&mut self.segment),
        };
        self.state = State::Silence;
        self.silence_run_ms = 0;
        segment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silence(frame_len: usize, num_frames: usize) -> Vec<f32> {
        vec![0.0; frame_len * num_frames]
    }

    fn loud_tone(frame_len: usize, num_frames: usize) -> Vec<f32> {
        (0..frame_len * num_frames)
            .map(|i| if i % 2 == 0 { 0.6 } else { -0.6 })
            .collect()
    }

    fn test_config() -> VadConfig {
        VadConfig {
            sample_rate: 16_000,
            frame_ms: 20,
            energy_threshold: 0.02,
            min_silence_ms: 100,
            max_segment_ms: 10_000,
        }
    }

    #[test]
    fn 無音発話無音のストリームからセグメントが1つ切り出される() {
        let config = test_config();
        let frame_len = 320; // 16000 * 20ms / 1000
        let mut vad = VadSegmenter::new(config);

        let mut samples = Vec::new();
        samples.extend(silence(frame_len, 5)); // 100ms 無音
        samples.extend(loud_tone(frame_len, 10)); // 200ms 発話
        samples.extend(silence(frame_len, 10)); // 200ms 無音（min_silence_ms=100msを超える）

        let segments = vad.push(&samples);

        assert_eq!(segments.len(), 1);
        let segment = &segments[0];
        assert_eq!(segment.start_ms, 100);
        assert!(!segment.samples.is_empty());
        assert!(segment.end_ms > segment.start_ms);
    }

    #[test]
    fn 発話が続く場合は最大長で強制的に分割される() {
        let config = test_config(); // max_segment_ms = 10_000
        let frame_len = 320;
        let mut vad = VadSegmenter::new(config);

        // 15秒分（750フレーム）連続で発話が続くストリーム
        let samples = loud_tone(frame_len, 750);

        let segments = vad.push(&samples);

        assert!(
            !segments.is_empty(),
            "10秒を超える発話は少なくとも1回は強制分割されること"
        );
        for segment in &segments {
            let duration_ms = segment.end_ms - segment.start_ms;
            assert!(
                duration_ms <= config.max_segment_ms as i64,
                "セグメント長({duration_ms}ms)が上限({}ms)を超えている",
                config.max_segment_ms
            );
        }
    }

    #[test]
    fn 無音のみのストリームではセグメントが切り出されない() {
        let config = test_config();
        let frame_len = 320;
        let mut vad = VadSegmenter::new(config);

        let samples = silence(frame_len, 20);
        let segments = vad.push(&samples);

        assert!(segments.is_empty());
    }
}
