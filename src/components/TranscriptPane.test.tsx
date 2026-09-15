// TranscriptPane コンポーネントのテストです。
// `@tauri-apps/api/event` の listen をモックし、`transcript://event` イベント受信時の
// 表示（partial/finalのスタイル差異・同一segmentIdの上書き・時系列順表示）を検証します。
import { render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { listen } from '@tauri-apps/api/event'
import TranscriptPane from './TranscriptPane'
import type { TranscriptEvent } from '../types/transcript'

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}))

const mockedListen = vi.mocked(listen)

/** テスト用の TranscriptEvent を組み立てます。未指定項目にはデフォルト値を使います。 */
function 文字起こしイベント(overrides: Partial<TranscriptEvent>): TranscriptEvent {
  return {
    segmentId: 'seg-1',
    startMs: 0,
    endMs: 1000,
    text: 'こんにちは',
    isFinal: true,
    engine: 'whisper-local',
    ...overrides,
  }
}

describe('TranscriptPane', () => {
  let transcriptHandler: ((event: { payload: TranscriptEvent }) => void) | undefined

  beforeEach(() => {
    mockedListen.mockReset()
    transcriptHandler = undefined
    mockedListen.mockImplementation(async (eventName, handler) => {
      if (eventName === 'transcript://event') {
        transcriptHandler = handler as (event: { payload: TranscriptEvent }) => void
      }
      return () => {}
    })
  })

  it('finalイベントは通常表示になる', async () => {
    render(<TranscriptPane />)
    await waitFor(() => expect(transcriptHandler).toBeDefined())

    transcriptHandler?.({
      payload: 文字起こしイベント({ segmentId: 'seg-1', text: '配信開始します', isFinal: true }),
    })

    const item = await screen.findByText('配信開始します')
    expect(item).toHaveAttribute('data-final', 'true')
  })

  it('partialイベントは淡色表示（data-final=false）になる', async () => {
    render(<TranscriptPane />)
    await waitFor(() => expect(transcriptHandler).toBeDefined())

    transcriptHandler?.({
      payload: 文字起こしイベント({ segmentId: 'seg-2', text: '今日は', isFinal: false }),
    })

    const item = await screen.findByText('今日は')
    expect(item).toHaveAttribute('data-final', 'false')
  })

  it('同一segmentIdのイベントは上書きされ、重複表示されない', async () => {
    render(<TranscriptPane />)
    await waitFor(() => expect(transcriptHandler).toBeDefined())

    transcriptHandler?.({
      payload: 文字起こしイベント({ segmentId: 'seg-3', text: '今日', isFinal: false }),
    })
    await screen.findByText('今日')

    transcriptHandler?.({
      payload: 文字起こしイベント({ segmentId: 'seg-3', text: '今日は良い天気です', isFinal: true }),
    })

    const item = await screen.findByText('今日は良い天気です')
    expect(item).toHaveAttribute('data-final', 'true')
    expect(screen.queryByText('今日')).not.toBeInTheDocument()
  })

  it('複数の発話区間が開始時刻順に表示される', async () => {
    render(<TranscriptPane />)
    await waitFor(() => expect(transcriptHandler).toBeDefined())

    transcriptHandler?.({
      payload: 文字起こしイベント({ segmentId: 'seg-later', startMs: 5000, text: '後半の発話' }),
    })
    transcriptHandler?.({
      payload: 文字起こしイベント({ segmentId: 'seg-earlier', startMs: 1000, text: '前半の発話' }),
    })

    const items = await screen.findAllByRole('listitem')
    expect(items.map((el) => el.textContent)).toEqual(['前半の発話', '後半の発話'])
  })
})
