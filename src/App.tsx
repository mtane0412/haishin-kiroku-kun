/**
 * アプリケーションのルートコンポーネントです。
 * 配信セッションの開始・終了・一覧表示を行う SessionBar、
 * マイク入力デバイスの選択・キャプチャ制御を行う DeviceSelector、
 * 文字起こし結果を表示する TranscriptPane を描画します。
 */
import SessionBar from './components/SessionBar'
import DeviceSelector from './components/DeviceSelector'
import TranscriptPane from './components/TranscriptPane'
import './App.css'

function App() {
  return (
    <>
      <SessionBar />
      <DeviceSelector />
      <TranscriptPane />
    </>
  )
}

export default App
