/**
 * 配信セッションの開始・終了・一覧表示を行うコンポーネントです。
 * Rust 側の `start_session` / `end_session` / `list_sessions` コマンドを
 * `invoke` で呼び出し、DB に永続化されたセッション情報を操作します。
 */
import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Session } from '../types/session'

/** 一覧取得時に指定する最大件数です。 */
const SESSION_LIST_LIMIT = 20

/** epochミリ秒を日本語ロケールの日時文字列に整形します。 */
function formatDateTime(epochMs: number): string {
  return new Date(epochMs).toLocaleString('ja-JP')
}

/** エラーオブジェクトからユーザー表示用のメッセージ文字列を取り出します。 */
function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message
  }
  return String(error)
}

function SessionBar() {
  const [sessions, setSessions] = useState<Session[]>([])
  const [title, setTitle] = useState('')
  const [error, setError] = useState<string | null>(null)

  const activeSession = sessions.find((session) => session.endedAt === null) ?? null

  // マウント時にセッション一覧を取得します。コンポーネントがアンマウントされた後に
  // 応答が届いた場合は setState を呼ばないよう `ignore` フラグで防ぎます。
  useEffect(() => {
    let ignore = false
    invoke<Session[]>('list_sessions', { limit: SESSION_LIST_LIMIT })
      .then((result) => {
        if (!ignore) {
          setSessions(result)
          setError(null)
        }
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

  async function handleStart(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault()
    try {
      const created = await invoke<Session>('start_session', { title })
      setSessions((prev) => [created, ...prev])
      setTitle('')
      setError(null)
    } catch (e) {
      setError(toErrorMessage(e))
    }
  }

  async function handleEnd() {
    if (!activeSession) {
      return
    }
    try {
      const ended = await invoke<Session>('end_session', { sessionId: activeSession.id })
      setSessions((prev) => prev.map((session) => (session.id === ended.id ? ended : session)))
      setError(null)
    } catch (e) {
      setError(toErrorMessage(e))
    }
  }

  return (
    <main className="container">
      <h1>haishin-kiroku-kun</h1>

      {error && <p role="alert">{error}</p>}

      {activeSession ? (
        <section>
          <p>
            配信中: {activeSession.title}（開始: {formatDateTime(activeSession.startedAt)}）
          </p>
          <button type="button" onClick={() => void handleEnd()}>
            配信を終了
          </button>
        </section>
      ) : (
        <form onSubmit={(e) => void handleStart(e)}>
          <label htmlFor="session-title">セッションタイトル</label>
          <input
            id="session-title"
            value={title}
            onChange={(e) => setTitle(e.currentTarget.value)}
            placeholder="配信タイトルを入力"
          />
          <button type="submit">配信を開始</button>
        </form>
      )}

      <h2>セッション一覧</h2>
      <ul>
        {sessions.map((session) => (
          <li key={session.id}>
            {session.title}（開始: {formatDateTime(session.startedAt)} / 終了:{' '}
            {session.endedAt === null ? '進行中' : formatDateTime(session.endedAt)}）
          </li>
        ))}
      </ul>
    </main>
  )
}

export default SessionBar
