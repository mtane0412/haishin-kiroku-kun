// DeviceSelector コンポーネントのテストです。
// `@tauri-apps/api/core` の invoke と `@tauri-apps/api/event` の listen をモックし、
// デバイス一覧表示・キャプチャ開始/停止・レベルメータ更新・エラー表示の振る舞いを検証します。
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import DeviceSelector from './DeviceSelector'
import type { InputDeviceInfo } from '../types/audio'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}))

const mockedInvoke = vi.mocked(invoke)
const mockedListen = vi.mocked(listen)

const デバイス一覧: InputDeviceInfo[] = [
  { name: 'MacBook Pro のマイク', isDefault: true },
  { name: 'USBマイク', isDefault: false },
]

describe('DeviceSelector', () => {
  beforeEach(() => {
    mockedInvoke.mockReset()
    mockedListen.mockReset()
    mockedListen.mockResolvedValue(vi.fn())
  })

  it('マウント時にデバイス一覧が表示され、既定デバイスが選択される', async () => {
    mockedInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'list_input_devices') {
        return デバイス一覧
      }
      throw new Error(`予期しないコマンド: ${cmd}`)
    })

    render(<DeviceSelector />)

    const select = await screen.findByLabelText('入力デバイス')
    expect(select).toHaveValue('MacBook Pro のマイク')
    expect(screen.getByRole('option', { name: 'USBマイク' })).toBeInTheDocument()
  })

  it('開始ボタンを押すと選択中のデバイス名で start_audio_capture が呼ばれる', async () => {
    const user = userEvent.setup()
    mockedInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === 'list_input_devices') {
        return デバイス一覧
      }
      if (cmd === 'start_audio_capture') {
        expect(args).toEqual({ deviceName: 'USBマイク' })
        return undefined
      }
      throw new Error(`予期しないコマンド: ${cmd}`)
    })

    render(<DeviceSelector />)

    await user.selectOptions(await screen.findByLabelText('入力デバイス'), 'USBマイク')
    await user.click(screen.getByRole('button', { name: 'キャプチャ開始' }))

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith('start_audio_capture', { deviceName: 'USBマイク' }),
    )
    expect(await screen.findByRole('button', { name: 'キャプチャ停止' })).toBeInTheDocument()
  })

  it('停止ボタンを押すと stop_audio_capture が呼ばれる', async () => {
    const user = userEvent.setup()
    mockedInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'list_input_devices') {
        return デバイス一覧
      }
      if (cmd === 'start_audio_capture' || cmd === 'stop_audio_capture') {
        return undefined
      }
      throw new Error(`予期しないコマンド: ${cmd}`)
    })

    render(<DeviceSelector />)

    await user.click(await screen.findByRole('button', { name: 'キャプチャ開始' }))
    await screen.findByRole('button', { name: 'キャプチャ停止' })
    await user.click(screen.getByRole('button', { name: 'キャプチャ停止' }))

    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith('stop_audio_capture'))
    expect(await screen.findByRole('button', { name: 'キャプチャ開始' })).toBeInTheDocument()
  })

  it('audio://level イベントを受信するとレベルメータの値が更新される', async () => {
    let levelHandler: ((event: { payload: { rms: number } }) => void) | undefined
    mockedListen.mockImplementation(async (eventName, handler) => {
      if (eventName === 'audio://level') {
        levelHandler = handler as (event: { payload: { rms: number } }) => void
      }
      return () => {}
    })
    mockedInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'list_input_devices') {
        return デバイス一覧
      }
      throw new Error(`予期しないコマンド: ${cmd}`)
    })

    render(<DeviceSelector />)
    await screen.findByLabelText('入力デバイス')

    await waitFor(() => expect(levelHandler).toBeDefined())
    levelHandler?.({ payload: { rms: 0.42 } })

    const meter = await screen.findByRole('meter')
    await waitFor(() => expect(meter).toHaveAttribute('aria-valuenow', '0.42'))
  })

  it('invoke が失敗した場合にエラーメッセージが表示される', async () => {
    mockedInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'list_input_devices') {
        throw new Error('デバイス一覧の取得に失敗しました')
      }
      throw new Error(`予期しないコマンド: ${cmd}`)
    })

    render(<DeviceSelector />)

    expect(await screen.findByRole('alert')).toHaveTextContent('デバイス一覧の取得に失敗しました')
  })
})
