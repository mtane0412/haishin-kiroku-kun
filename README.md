# mic-app

配信者と AI エージェントをつなぐデスクトップアプリです（Tauri v2 / Windows・macOS 対応）。

## 機能

- 選択したマイク入力のリアルタイム文字起こし（ローカル whisper.cpp / OpenAI Realtime API / Deepgram Streaming から選択可能、OBS とのマイク共有に対応）
- Twitch チャットビューワー兼チャットログ保存
- 音声文字起こしとチャットを AI エージェントへ渡す localhost REST + SSE API
- 配信セッション単位での文字起こし・チャットログの永続保存とエクスポート

実装方針の詳細は計画書（`win-mac-tauri-ai-mic-obs-linked-coral.md`）を参照してください。

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 開発

```bash
npm install
npm run tauri dev
```

### 品質チェック

```bash
npm run lint        # ESLint
npm run type-check  # tsc --noEmit
npm run test         # vitest run
cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings
```
