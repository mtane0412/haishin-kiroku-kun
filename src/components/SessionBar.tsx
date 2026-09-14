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
  // 初回のセッション一覧取得が完了したかどうかです。完了前に開始フォームを表示すると、
  // 取得結果が後から届いて新しく開始したセッションの表示を上書きしてしまう恐れがあるため、
  // 完了するまで開始フォームの表示を待ちます。
  const [isInitialLoadDone, setIsInitialLoadDone] = useState(false)
  // start_session / end_session の応答待ち中かどうかです。
  // 応答が返るまでボタンを無効化し、二重送信を防ぎます。
  const [isPending, setIsPending] = useState(false)

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
      .finally(() => {
        if (!ignore) {
          setIsInitialLoadDone(true)
        }
      })
    return () => {
      ignore = true
    }
  }, [])

  async function handleStart(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault()
    if (isPending) {
      return
    }
    setIsPending(true)
    try {
      const created = await invoke<Session>('start_session', { title })
      setSessions((prev) => [created, ...prev])
      setTitle('')
      setError(null)
    } catch (e) {
      setError(toErrorMessage(e))
    } finally {
      setIsPending(false)
    }
  }

  async function handleEnd() {
    if (!activeSession || isPending) {
      return
    }
    setIsPending(true)
    try {
      const ended = await invoke<Session>('end_session', { sessionId: activeSession.id })
      setSessions((prev) => prev.map((session) => (session.id === ended.id ? ended : session)))
      setError(null)
    } catch (e) {
      setError(toErrorMessage(e))
    } finally {
      setIsPending(false)
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
          <button type="button" onClick={() => void handleEnd()} disabled={isPending}>
            配信を終了
          </button>
        </section>
      ) : isInitialLoadDone ? (
        <form onSubmit={(e) => void handleStart(e)}>
          <label htmlFor="session-title">セッションタイトル</label>
          <input
            id="session-title"
            value={title}
            onChange={(e) => setTitle(e.currentTarget.value)}
            placeholder="配信タイトルを入力"
          />
          <button type="submit" disabled={isPending}>
            配信を開始
          </button>
        </form>
      ) : (
        <p>読み込み中...</p>
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
