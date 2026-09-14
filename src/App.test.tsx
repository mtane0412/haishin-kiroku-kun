// App コンポーネントのスモークテストです。SessionBar が描画されることのみを確認します。
// SessionBar / DeviceSelector 自体の詳細な振る舞いはそれぞれの *.test.tsx で検証します。
// DeviceSelector が `@tauri-apps/api/event` の listen を呼び出すため、invoke に加えて
// listen もモックし、未モックのTauri内部呼び出しによる警告・失敗を防ぎます。
import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import App from './App'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
}))

describe('App', () => {
  it('見出しが表示される', async () => {
    render(<App />)
    expect(
      await screen.findByRole('heading', { name: 'haishin-kiroku-kun', level: 1 }),
    ).toBeInTheDocument()
  })
})
