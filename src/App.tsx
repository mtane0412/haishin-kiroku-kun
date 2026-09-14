/**
 * アプリケーションのルートコンポーネントです。
 * フェーズ1時点では配信セッションの開始・終了・一覧表示を行う SessionBar のみを描画します。
 */
import SessionBar from './components/SessionBar'
import './App.css'

function App() {
  return <SessionBar />
}

export default App
