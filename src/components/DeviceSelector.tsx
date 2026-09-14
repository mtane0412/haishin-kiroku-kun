/**
 * マイク入力デバイスの選択・キャプチャ開始/停止・レベルメータ表示を行うコンポーネントです。
 * Rust 側の `list_input_devices` / `start_audio_capture` / `stop_audio_capture` コマンドを
 * `invoke` で呼び出し、`audio://level` イベントを購読してRMSレベルを表示します。
 */
import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { InputDeviceInfo } from '../types/audio'

/** `audio://level` イベントのペイロードです。 */
interface AudioLevelPayload {
  rms: number
}

/** エラーオブジェクトからユーザー表示用のメッセージ文字列を取り出します。 */
function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message
  }
  return String(error)
}

function DeviceSelector() {
  const [devices, setDevices] = useState<InputDeviceInfo[]>([])
  const [selectedDevice, setSelectedDevice] = useState('')
  const [isCapturing, setIsCapturing] = useState(false)
  const [level, setLevel] = useState(0)
  const [error, setError] = useState<string | null>(null)

  // マウント時にデバイス一覧を取得し、既定デバイスを初期選択します。
  useEffect(() => {
    let ignore = false
    invoke<InputDeviceInfo[]>('list_input_devices')
      .then((result) => {
        if (ignore) {
          return
        }
        setDevices(result)
        const defaultDevice = result.find((d) => d.isDefault) ?? result[0]
        if (defaultDevice) {
          setSelectedDevice(defaultDevice.name)
        }
        setError(null)
      })
      .catch((e: unknown) => {
        if (!ignore) {
          setError(toErrorMessage(e))
        }
      })
    return () => {
      ignore = true
    }
  }, [])

  // レベルメータ用にRMSイベントを購読します。
  useEffect(() => {
    let unlisten: (() => void) | undefined
    let ignore = false
    listen<AudioLevelPayload>('audio://level', (event) => {
      setLevel(event.payload.rms)
    })
      .then((fn) => {
        if (ignore) {
          fn()
          return
        }
        unlisten = fn
      })
      .catch((e: unknown) => {
        if (!ignore) {
          setError(toErrorMessage(e))
        }
      })
    return () => {
      ignore = true
      unlisten?.()
    }
  }, [])

  async function handleStart() {
    try {
      await invoke('start_audio_capture', { deviceName: selectedDevice })
      setIsCapturing(true)
      setError(null)
    } catch (e) {
      setError(toErrorMessage(e))
    }
  }

  async function handleStop() {
    try {
      await invoke('stop_audio_capture')
      setIsCapturing(false)
      setLevel(0)
      setError(null)
    } catch (e) {
      setError(toErrorMessage(e))
    }
  }

  return (
    <section>
      <h2>マイク入力</h2>

      {error && <p role="alert">{error}</p>}

      <label htmlFor="input-device-select">入力デバイス</label>
      <select
        id="input-device-select"
        value={selectedDevice}
        onChange={(e) => setSelectedDevice(e.currentTarget.value)}
        disabled={isCapturing}
      >
        {devices.map((device) => (
          <option key={device.name} value={device.name}>
            {device.name}
            {device.isDefault ? '（既定）' : ''}
          </option>
        ))}
      </select>

      {isCapturing ? (
        <button type="button" onClick={() => void handleStop()}>
          キャプチャ停止
        </button>
      ) : (
        <button type="button" onClick={() => void handleStart()} disabled={!selectedDevice}>
          キャプチャ開始
        </button>
      )}

      <div
        role="meter"
        aria-label="入力レベル"
        aria-valuemin={0}
        aria-valuemax={1}
        aria-valuenow={level}
      >
        <div style={{ width: `${Math.min(level, 1) * 100}%` }} />
      </div>
    </section>
  )
}

export default DeviceSelector
