// App コンポーネントのスモークテストです。SessionBar が描画されることのみを確認します。
// SessionBar 自体の詳細な振る舞いは SessionBar.test.tsx で検証します。
import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import App from './App'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue([]),
}))

describe('App', () => {
  it('見出しが表示される', async () => {
    render(<App />)
    expect(
      await screen.findByRole('heading', { name: 'haishin-kiroku-kun', level: 1 }),
    ).toBeInTheDocument()
  })
})
