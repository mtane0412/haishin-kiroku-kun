// App コンポーネントのスモークテストです。フェーズ0時点ではテスト基盤（vitest +
// Testing Library）の疎通確認が目的であり、本格的な機能テストはフェーズ1以降で
// SessionBar 等の実コンポーネントに対して追加します。
import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import App from './App'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

describe('App', () => {
  it('見出しが表示される', () => {
    render(<App />)
    expect(
      screen.getByRole('heading', { name: /Welcome to Tauri \+ React/i }),
    ).toBeInTheDocument()
  })
})
