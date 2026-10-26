# 変更ログ

形式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/)、日付は JST。

## [0.2.0] - 2026-10-26

### 追加

- 設定の画面（Ctrl+,）: 一般・テーマ・キー・詳細・このアプリについて。common.toml の言語・倍率・テーマに従い、ほかの uchmk のアプリと揃う。
- キー一覧（F1）: keymap のキーと、keymap より先に読むキー（Esc・Ctrl+Shift+P・Ctrl+= / Ctrl+-・Ctrl+0・F12）。
- `keymap.toml` でキーを変えられる（既定は Ctrl+, 設定・F1 キー一覧・Ctrl+Q 終了）。直すとすぐ効き、書き間違いは通知に出る。
- コマンド パレットに設定・キー一覧・不具合の報告・倍率を戻す・終了。
- 日本語の字体。
- `--keys` の台本（`FUDE_KEYS_DONE` に終わり方を書く）。

### 変更

- 窓の場所は閉じるときにも書く。

## [0.1.0] - 2026-10-26

### 追加

- ito の `templates/app` から作った（窓・テーマ・言語・倍率・F12 の不具合報告・`fude.com`）。
