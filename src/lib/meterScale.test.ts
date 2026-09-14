// meterScale のテストです。
// マイクのRMS値（振幅、0.0〜1.0程度）をレベルメーター表示用の0〜1の比率へ
// dB（対数）スケールで変換する `rmsToMeterRatio` の振る舞いを検証します。
// 音声は線形スケールだと実用的な音量のほとんどが0付近に圧縮されて見えなくなるため、
// 対数スケールへの変換が必須です。
import { describe, expect, it } from 'vitest'
import { METER_FLOOR_DB, rmsToMeterRatio } from './meterScale'

describe('rmsToMeterRatio', () => {
  it('無音（0）の場合は比率0になる', () => {
    expect(rmsToMeterRatio(0)).toBe(0)
  })

  it('フルスケール（1.0）の場合は比率1になる', () => {
    expect(rmsToMeterRatio(1)).toBeCloseTo(1, 5)
  })

  it('下限dB（METER_FLOOR_DB）に相当する振幅では比率がほぼ0になる', () => {
    const floorAmplitude = 10 ** (METER_FLOOR_DB / 20)
    expect(rmsToMeterRatio(floorAmplitude)).toBeCloseTo(0, 5)
  })

  it('典型的な発話レベル（0.005前後）は線形スケールでは0.5%だが、対数スケールでは目視できる比率になる', () => {
    const ratio = rmsToMeterRatio(0.005)
    // 線形換算(0.5%)より十分大きく、かつ0〜1の範囲に収まること
    expect(ratio).toBeGreaterThan(0.15)
    expect(ratio).toBeLessThanOrEqual(1)
  })

  it('下限dBを下回るごく小さい値は比率0にクランプされる（負の比率にならない）', () => {
    expect(rmsToMeterRatio(0.0000001)).toBe(0)
  })

  it('振幅が大きいほど比率も大きくなる（単調増加）', () => {
    const small = rmsToMeterRatio(0.005)
    const medium = rmsToMeterRatio(0.05)
    const large = rmsToMeterRatio(0.5)
    expect(medium).toBeGreaterThan(small)
    expect(large).toBeGreaterThan(medium)
  })
})
