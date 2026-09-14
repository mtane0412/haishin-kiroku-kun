/**
 * マイク入力RMS値をレベルメーター表示用の比率（0〜1）へ変換するユーティリティです。
 * 音声は線形スケール（振幅そのまま）で表示すると、通常の発話音量（振幅0.01〜0.1程度）が
 * フルスケール（1.0）に対してごくわずかな範囲に圧縮され、視覚的にほぼ動きが見えなくなります。
 * そのため人間の音量知覚やオーディオVUメーターの慣習に合わせ、dB（対数）スケールへ変換します。
 */

/** メーターの下限とするdBFS値。この値以下の振幅は比率0（メーター最下点）として扱います。 */
export const METER_FLOOR_DB = -60

/**
 * RMS振幅（0.0〜1.0程度、フルスケールが1.0）をメーター表示用の比率（0〜1）に変換します。
 * `rms <= 0` の場合は無音として比率0を返します。
 */
export function rmsToMeterRatio(rms: number): number {
  if (rms <= 0) {
    return 0
  }
  const db = 20 * Math.log10(rms)
  const clampedDb = Math.max(METER_FLOOR_DB, Math.min(0, db))
  return (clampedDb - METER_FLOOR_DB) / -METER_FLOOR_DB
}
