/**
 * 文字起こし結果を表示するコンポーネントです。
 * Rust 側から送出される `transcript://event` イベントを購読し、発話区間（segmentId）
 * ごとに最新の結果を保持します。partial（途中経過）は淡色、final（確定）は通常表示にします。
 */
import { useEffect, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import type { TranscriptEvent } from '../types/transcript'

/** `transcript://event` イベント名です。Rust側 `commands` 層が送出する想定です。 */
const TRANSCRIPT_EVENT = 'transcript://event'

function TranscriptPane() {
  // segmentId をキーに最新イベントを保持する。partial→finalの更新は同じsegmentIdで
  // 上書きされるため、同一区間が重複表示されることはない。
  const [eventsById, setEventsById] = useState<Map<string, TranscriptEvent>>(new Map())

  useEffect(() => {
    let unlisten: (() => void) | undefined
    let ignore = false

    listen<TranscriptEvent>(TRANSCRIPT_EVENT, (event) => {
      setEventsById((prev) => {
        const next = new Map(prev)
        next.set(event.payload.segmentId, event.payload)
        return next
      })
    }).then((fn) => {
      if (ignore) {
        fn()
        return
      }
      unlisten = fn
    })

    return () => {
      ignore = true
      unlisten?.()
    }
  }, [])

  const sortedEvents = Array.from(eventsById.values()).sort((a, b) => a.startMs - b.startMs)

  return (
    <section>
      <h2>文字起こし</h2>
      <ul>
        {sortedEvents.map((event) => (
          <li key={event.segmentId} data-final={event.isFinal} className={event.isFinal ? 'final' : 'partial'}>
            {event.text}
          </li>
        ))}
      </ul>
    </section>
  )
}

export default TranscriptPane
