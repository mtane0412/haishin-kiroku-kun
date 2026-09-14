/**
 * アプリケーションのルートコンポーネントです。
 * 配信セッションの開始・終了・一覧表示を行う SessionBar と、
 * マイク入力デバイスの選択・キャプチャ制御を行う DeviceSelector を描画します。
 */
import SessionBar from './components/SessionBar'
import DeviceSelector from './components/DeviceSelector'
import './App.css'

function App() {
  return (
    <>
      <SessionBar />
      <DeviceSelector />
    </>
  )
}

export default App
