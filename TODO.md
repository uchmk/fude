# TODO

雛形（ito の `templates/app`）から作った。雛形で動いているのは、窓（場所を覚える）、common.toml のテーマ・言語・倍率、
`--version` / `--help`、F12 の不具合報告、`fude.com`、アイコンと版の情報、TESTING の表の生成まで。
下は**依存は入っているが、まだ配線していないもの**。先に作った別のアプリが手本になる。自前で書かずに ito のクレートを使う
（足りない所は ito に頼む。`scripts/ito-first.py` が見ている）。

- [x] `--keys` の台本: `ito-keyscript`（`Dialect`・`Runner`・`Press`）。手本は michi の `main.rs`。`lanes.conf` の `keys_env` と合わせる。
- [x] `keymap.toml` と設定の「キー」ページ: `ito-keys`・`ito-keymap`・`ito-prefs` の `keys_page`。既定のキーは `defaults/keymap.toml`
  （`<C-,>` 設定・`<F1>` ヘルプ・`<C-q>` 終了）。手本は minato の `keymap.rs`・`prefs.rs`。
- [x] F1 のキー一覧: `ito-help`（`ito-keymap` の一覧を描く）。手本は michi の `app.rs`・`ui.rs`。
- [ ] 通知と設定の「通知」ページ: `ito-notify`（システムの通知・音・webhook）と `ito-prefs` の `notify_page`。手本は minato の `app.rs`・`settings.rs`。
- [x] 設定の画面: `ito_prefs::show`（Language・CLOCK・Theme の共通のページ + キー）。手本は minato の `settings.rs`。
- [x] 窓の場所を戻す: 雛形は `window::Saved` で戻し `Tracker::follow` で書く。終わるときに `Tracker::close`。
- [ ] 通知を入れる: 設定の読み違い・keymap の警告はベルに出る（0.2.0）。`ito_notices::Notice` を push し（`go` に飛び先の型）、`ito-notify` の `Toast::open` で押すと開くトーストにする。
- [ ] コマンドパレットの中身: 設定・キー一覧・不具合報告・倍率・終了まで（0.2.0）。`Command` と `commands` に、アプリのコマンドを足す（キーは `ito-keymap` から引く）。手本は minato の `main.rs`。
- [x] 字体（日本語が出る字体を `egui` に足す）。手本は minato の `fonts.rs`。
- [ ] `TESTING.md` に確認の行を足し、`cargo run --example make-testcheck` で `TESTING-CHECKS.md` を作る。
- [ ] `icon.rs` に本当のアイコンを描く。
- [ ] リリースの workflow は kura か yagura のものを写す。

## 進め方（版ごと）

モック（`C:\dev\fude-mock\index.html`）が仕様。1 つ終わるごとに `scripts/verify.sh` が ALL OK → 版を上げて CHANGELOG と一緒にコミット。

- [x] 0.2.0 土台: 文書（.txt）と状態（TOML）の決まりを CLAUDE.md に、`--keys`、keymap、ヘルプ、設定の画面の枠、字体、`Tracker::close`。
- [x] 0.3.0 書く帯: 横・縦の帯、←→ で直す、Enter の段落、貼り付け、上限の帯と時計、書きかけの自動保存と続き、消せないモード。
- [x] 0.4.0 清書: `layout/`（原稿用紙・罫線・方眼・無地、縦の字形）と `draw.rs`、Esc で清書・Enter で続き、マスのクリックで直す、ルビと傍点。
- [x] 0.5.0 ノート: 綴じる、本棚と篇の一覧、清書で ←→ でめくる、ノートごとの設定、昔の篇を開いて直す。
- [x] 0.6.0 書くのを助けるもの: 設定の画面を全部、お題、記録（1 日の字数・続けた日数・1 分の字数）、速さの線。
- [ ] 仕上げ: ヘルプの中身、パレットのコマンド、TESTING.md の行、README、アイコン。

## ito に頼んだもの（来るまで fude に代わりを書かない）

全部入った（#38・#39・#41・#42 は v0.114.0、#40 は v0.115.0 `f916fd6`）。使う段で rev を上げて（`scripts/lanes.sh sync`）配線する。

- [x] [uchmk/ito#38](https://github.com/uchmk/ito/issues/38) 明朝の字体 `ito_common::fonts::MINCHO` → 0.4.0 清書の既定を明朝に。
- [x] [uchmk/ito#39](https://github.com/uchmk/ito/issues/39) `ito_common::documents_dir()` → 0.5.0 ノートの置き場の既定 `ドキュメント\fude`。
- [x] [uchmk/ito#40](https://github.com/uchmk/ito/issues/40) `ito-ime` と keyscript の `<Preedit>` `<Commit>` → 0.3.0 書く帯で日本語を打つ。
  縦の帯の候補窓の位置は実機の IME で確かめる。
- [x] [uchmk/ito#41](https://github.com/uchmk/ito/issues/41) 短い音 → 0.6.0 `sound.rs`（打つ音）と設定の行。
- [x] [uchmk/ito#42](https://github.com/uchmk/ito/issues/42) `ito-export` → 0.6.0 / 仕上げ `svg.rs`・`export.rs`（PNG・PDF・印刷、1 篇・1 冊）。

## ito に頼んでいるもの（0.4.0 から。来るまで ito-prefs の今あるもので代える）

- [ ] [uchmk/ito#43](https://github.com/uchmk/ito/issues/43) 区切りのボタン（seg） → 清書の道具の列（紙・向き・マス・1 行）。今は `ito_prefs::select`。
- [ ] [uchmk/ito#44](https://github.com/uchmk/ito/issues/44) 塗ったボタン（primary・danger） → 清書の帯の「続きを書く」。今は `ito_prefs::button`。
