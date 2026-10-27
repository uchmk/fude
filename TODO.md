# TODO

雛形（ito の `templates/app`）から作った。雛形で動いているのは、窓（場所を覚える）、common.toml のテーマ・言語・倍率、
`--version` / `--help`、F12 の不具合報告、`fude.com`、アイコンと版の情報、TESTING の表の生成まで。
下は**依存は入っているが、まだ配線していないもの**。先に作った別のアプリが手本になる。自前で書かずに ito のクレートを使う
（足りない所は ito に頼む。`scripts/ito-first.py` が見ている）。

- [x] `--keys` の台本: `ito-keyscript`（`Dialect`・`Runner`・`Press`）。手本は michi の `main.rs`。`lanes.conf` の `keys_env` と合わせる。
- [x] `keymap.toml` と設定の「キー」ページ: `ito-keys`・`ito-keymap`・`ito-prefs` の `keys_page`。既定のキーは `defaults/keymap.toml`
  （`<C-,>` 設定・`<F1>` ヘルプ・`<C-q>` 終了）。手本は minato の `keymap.rs`・`prefs.rs`。
- [x] F1 のキー一覧: `ito-help`（`ito-keymap` の一覧を描く）。手本は michi の `app.rs`・`ui.rs`。
- [x] 通知と設定の「通知」ページ: `ito-notify`（システムの通知・音・webhook）と `ito-prefs` の `notify_page`・`NotifyCfg`
  （[uchmk/ito#46](https://github.com/uchmk/ito/issues/46)、v0.119.2 `ef71a41`）。`notify.rs`。上限に届いたとき、窓が後ろにあれば知らせる。
- [x] 設定の画面: `ito_prefs::show`（Language・CLOCK・Theme の共通のページ + キー）。手本は minato の `settings.rs`。
- [x] 窓の場所を戻す: 雛形は `window::Saved` で戻し `Tracker::follow` で書く。終わるときに `Tracker::close`。
- [x] 通知を入れる: 設定の読み違い・keymap の警告はベルに出る（0.2.0）。上限の知らせはシステムの通知に出る。押すと fude の窓が
  前に来る（`NotifyCfg::tell_open`・`notify_window`、[uchmk/ito#47](https://github.com/uchmk/ito/issues/47)、v0.120.0 `20056d2`）。
- [x] コマンドパレットの中身: 設定・キー一覧・不具合報告・倍率・終了（0.2.0）、清書・紙・縦横・コピー・新しく書く（0.4.0）、本棚・保存（0.5.0）、
  書き出し・印刷（0.8.0）、設定の「書く」「このノート」のページ（0.9.0）。
- [x] 字体（日本語が出る字体を `egui` に足す）。手本は minato の `fonts.rs`。
- [x] `TESTING.md` に確認の行を足し、`cargo run --example make-testcheck` で `TESTING-CHECKS.md` を作る。日本語は `scripts/testcheck-ja.toml`。
- [x] `icon.rs` に本当のアイコンを描く（朱の原稿用紙の 1 マスと筆の一画）。
- [x] リリースの workflow: `scripts/lanes.sh sync` が ito の雛形から書く（`lanes.conf` の `workflows=`）。

## 進め方（版ごと）

モック（`C:\dev\fude-mock\index.html`）が仕様。1 つ終わるごとに `scripts/verify.sh` が ALL OK → 版を上げて CHANGELOG と一緒にコミット。

- [x] 0.2.0 土台: 文書（.txt）と状態（TOML）の決まりを CLAUDE.md に、`--keys`、keymap、ヘルプ、設定の画面の枠、字体、`Tracker::close`。
- [x] 0.3.0 書く帯: 横・縦の帯、←→ で直す、Enter の段落、貼り付け、上限の帯と時計、書きかけの自動保存と続き、消せないモード。
- [x] 0.4.0 清書: `layout/`（原稿用紙・罫線・方眼・無地、縦の字形）と `draw.rs`、Esc で清書・Enter で続き、マスのクリックで直す、ルビと傍点。
- [x] 0.5.0 ノート: 綴じる、本棚と篇の一覧、清書で ←→ でめくる、ノートごとの設定、昔の篇を開いて直す。
- [x] 0.6.0 書くのを助けるもの: 設定の画面を全部、お題、記録（1 日の字数・続けた日数・1 分の字数）、速さの線。
- [x] 0.7.0 打つ音: タイプライター・鉛筆の音、上限の知らせの音、音の大きさ。
- [x] 0.8.0 書き出し: PNG・PDF・印刷（1 篇・1 冊）をノートの `export\` に。
- [x] 0.9.0 仕上げ: ヘルプの中身、パレットのコマンド、TESTING.md の行、README、アイコン。

## ito に頼んだもの（来るまで fude に代わりを書かない）

全部入った（#38・#39・#41・#42 は v0.114.0、#40 は v0.115.0 `f916fd6`）。使う段で rev を上げて（`scripts/lanes.sh sync`）配線する。

- [x] [uchmk/ito#38](https://github.com/uchmk/ito/issues/38) 明朝の字体 `ito_common::fonts::MINCHO` → 0.4.0 清書の既定を明朝に。
- [x] [uchmk/ito#39](https://github.com/uchmk/ito/issues/39) `ito_common::documents_dir()` → 0.5.0 ノートの置き場の既定 `ドキュメント\fude`。
- [x] [uchmk/ito#40](https://github.com/uchmk/ito/issues/40) `ito-ime` と keyscript の `<Preedit>` `<Commit>` → 0.3.0 書く帯で日本語を打つ。
  縦の帯の候補窓の位置は実機の IME で確かめる。
- [x] [uchmk/ito#41](https://github.com/uchmk/ito/issues/41) 短い音 → 0.7.0 `sound.rs`（打つ音）と設定の行。
- [x] [uchmk/ito#42](https://github.com/uchmk/ito/issues/42) `ito-export` → 0.8.0 `svg.rs`・`export.rs`（PNG・PDF・印刷、1 篇・1 冊）。

## ito に頼んでいるもの（0.4.0 から。来るまで ito-prefs の今あるもので代える）

- [x] [uchmk/ito#43](https://github.com/uchmk/ito/issues/43) 区切りのボタン（seg） → v0.117.0。清書の道具の列（紙・向き・マス・1 行）を `ito_prefs::seg` に。
- [x] [uchmk/ito#44](https://github.com/uchmk/ito/issues/44) 塗ったボタン（primary・danger） → v0.118.0 `ab21bf8`。清書の帯の「続きを書く」を `Tone::Primary` に。
- [x] [uchmk/ito#45](https://github.com/uchmk/ito/issues/45) 雛形の TESTING.md の形と `make-testcheck` の `src_dirs` → v0.116.0 `8eabb2a` に上げた（今は v0.120.0 `20056d2`）。
- [x] [uchmk/ito#46](https://github.com/uchmk/ito/issues/46) 共通の `[notify]` の読み書きと送る所 → v0.119.2 `ef71a41`。設定の「通知」ページと、上限の知らせをシステムの通知にも。
- [x] [uchmk/ito#47](https://github.com/uchmk/ito/issues/47) 押すと窓が前に来る通知 → v0.120.0 `20056d2`。上限の知らせに「開く」。
- [x] [uchmk/ito#48](https://github.com/uchmk/ito/issues/48) 共通のステータスバー → v0.122.0 `6c47896`。設定を除く全部の
  画面の下に（画面の名前・字数と時間・清書の形・キーの案内・設定・右端の時計）。打つ間はタイトルバーと一緒に薄く。

## 画面の確認（TESTING-CHECKS.md、2026-10-27）で見つけたもの

- [x] 設定でドロップダウンを開いたまま Esc → ドロップダウンだけでなく設定の画面まで閉じる → ito v0.119.3 `5ae5ca7`
  （`Keys::standard` が開いたポップアップに Esc を残す）で直った。fude は `state.busy(ui)` も渡す（欄にいる間の Esc）。
- [x] 清書の帯に「続きを書く」と Esc の案内が 2 つ並ぶ。長いコピーの知らせが折り返す → 道具の列のほうを外した。
  1280 の幅でコピーの知らせが 1 行に収まる。
- [x] キー一覧の本棚の組で、N が `n` と出る（keymap の書き方のまま） → 直さない。キー一覧は全部 keymap の書き方
  （`<C-n>`・`<F2>`）で、そこでは `n` が N のキー。`N` と書くと Shift+N になる。
- [x] F12 の報告の context が「Config: defaults only」だけ（`ito_bugreport` に設定のパスを渡していない） → ある設定の
  ファイル（common.toml・keymap.toml・config.toml）と倍率を渡す。
- [x] ベルを押しても通知の一覧が開かない → ito v0.116.1 `565a36e` で直った。
- [x] 設定の「お題を足す」の欄が Enter のあとフォーカスを失う → fude の側で `ito_prefs::field_id` にフォーカスを戻す。
- [x] `ito_prefs::seg` の選んだ所の字が `on_accent` でなく地の字の色のまま → ito v0.119.1 `e4569fa` で直った。
- [x] 設定の「このノート」で上限を「自分で決める」にすると、ラベルと説明が細い列に押し込まれて縦に長く折り返す
  → 数の欄と単位を行の下に出す（`limit_row`）。
- [x] 窓が狭い（733pt。いちばん小さい窓は 720pt）と、ステータスバーのキーの案内が「設定」と時計の下に潜って重なる。
  左の全部に 960pt ほど要る。`ito_statusbar::show` が左と右を別々に並べ、間を取らないため
  → [uchmk/ito#49](https://github.com/uchmk/ito/issues/49) が ito v0.122.1 `36a0171` で直った（右端を先に置き、
  左はその手前で「…」に切る）。733pt の書く・清書の画面で重ならないのを見た。
