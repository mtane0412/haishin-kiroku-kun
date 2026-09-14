// SessionBar コンポーネントのテストです。
// `@tauri-apps/api/core` の invoke をモックし、コマンド名ごとに戻り値を出し分けて
// セッション一覧表示・開始・終了・エラー表示の振る舞いを検証します。
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import SessionBar from './SessionBar'
import type { Session } from '../types/session'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockedInvoke = vi.mocked(invoke)

const 進行中セッション: Session = {
  id: 'session-1',
  title: '9月14日 ゲーム配信',
  startedAt: 1_700_000_000_000,
  endedAt: null,
  twitchChannel: null,
  engine: 'whisper-local',
}

describe('SessionBar', () => {
  beforeEach(() => {
    mockedInvoke.mockReset()
  })

  it('マウント時にセッション一覧が表示される', async () => {
    mockedInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'list_sessions') {
        return [進行中セッション]
      }
      throw new Error(`予期しないコマンド: ${cmd}`)
    })

    render(<SessionBar />)

    const list = await screen.findByRole('list')
    expect(await within(list).findByText(/9月14日 ゲーム配信/)).toBeInTheDocument()
  })

  it('タイトルを入力して開始ボタンを押すと start_session が呼ばれ、終了ボタンが表示される', async () => {
    const user = userEvent.setup()
    mockedInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === 'list_sessions') {
        return []
      }
      if (cmd === 'start_session') {
        expect(args).toEqual({ title: '雑談配信' })
        return { ...進行中セッション, title: '雑談配信' }
      }
      throw new Error(`予期しないコマンド: ${cmd}`)
    })

    render(<SessionBar />)

    // 初回のセッション一覧取得が完了し開始フォームが表示されるまで待つ
    await user.type(await screen.findByLabelText('セッションタイトル'), '雑談配信')
    await user.click(screen.getByRole('button', { name: '配信を開始' }))

    expect(await screen.findByRole('button', { name: '配信を終了' })).toBeInTheDocument()
    expect(mockedInvoke).toHaveBeenCalledWith('start_session', { title: '雑談配信' })
  })

  it('終了ボタンを押すと end_session が呼ばれ、一覧の表示が進行中から終了時刻に変わる', async () => {
    const user = userEvent.setup()
    const 終了後セッション: Session = { ...進行中セッション, endedAt: 1_700_000_100_000 }
    mockedInvoke.mockImplementation(async (cmd, args) => {
      if (cmd === 'list_sessions') {
        return [進行中セッション]
      }
      if (cmd === 'end_session') {
        expect(args).toEqual({ sessionId: 'session-1' })
        return 終了後セッション
      }
      throw new Error(`予期しないコマンド: ${cmd}`)
    })

    render(<SessionBar />)

    const list = await screen.findByRole('list')
    expect(await within(list).findByText(/進行中/)).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: '配信を終了' }))

    await waitFor(() => expect(within(list).queryByText(/進行中/)).not.toBeInTheDocument())
    expect(mockedInvoke).toHaveBeenCalledWith('end_session', { sessionId: 'session-1' })
  })

  it('invoke が失敗した場合にエラーメッセージが表示される', async () => {
    mockedInvoke.mockImplementation(async (cmd) => {
      if (cmd === 'list_sessions') {
        throw new Error('DB接続に失敗しました')
      }
      throw new Error(`予期しないコマンド: ${cmd}`)
    })

    render(<SessionBar />)

    expect(await screen.findByRole('alert')).toHaveTextContent('DB接続に失敗しました')
  })
})
