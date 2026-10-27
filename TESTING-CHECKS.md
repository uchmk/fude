# Checklist

54 checks in TESTING.md, 52 left for a person.

## 1. 起動

- [~] **1.1** 起動する → 1280x800 の窓が「fude」の題で開き、タスクバー（と Alt+Tab）に朱の枠と筆の一画のアイコン — *Run `fude` → A window of 1280x800 opens with the title "fude"; the taskbar (and `Alt+Tab`) show the icon, a vermilion square with a brush's stroke in it*
- [x] **1.2** 窓を動かし大きさを変えて閉じ、また起動する → 同じ場所と大きさで開く — *Move and resize the window, close it and start it again → The window comes back at the same place and size*
- [x] **1.3** F12 → ブラウザーで不具合報告のフォームが開き、版と OS が入っている — *Press `F12` → The browser opens the repository's new issue form with the version and OS filled in*
- [x] **1.4** `ドキュメント\fude` がない状態で起動 → `ドキュメント\fude\ノート\` ができ、何も聞かれずにすぐ打てる — *Start it with no `Documents\fude` folder → `Documents\fude\ノート\` is made and the band takes letters at once, with nothing to answer first*

## 2. 書く

自動テスト済みなので下には出していない: 2.3, 2.4

- [~] **2.1** IME で「きょうははれ」を打ち、変換して確定 → 変換中の字は帯の打つ所に出る。打つ所は動かず、字のほうが流れる — *Type `きょうははれ` with the IME on, convert it and confirm → The letters being converted stand at the input point on the band; the input point stays where it is and the letters flow away from it*
- [x] **2.2** ← を 2 回、X を打ち、Home と End → 1 字ずつ戻り、そこに X が入る。Home・End で端へ — *Press `Left` twice, type `X`, then `Home` and `End` → The input point goes back by a letter each time, `X` goes in there, and `Home` / `End` take it to the ends*
- [x] **2.5** タブと空の行 3 つを含む文を貼り付ける → タブは空白に、空の行は段落の区切り 1 つに — *Paste text with a tab and three blank lines in it → The tab becomes a space and the blank lines one paragraph break*
- [x] **2.6** 設定の「書く」で向きを縦に → 帯が縦になり、↑ ↓ で動く — *Settings, Writing: set the direction to vertical → The band stands upright and `Up` / `Down` move the input point*
- [~] **2.7** マウスに触れずにしばらく打ち、マウスを動かす → 打っている間はタイトルバーが薄くなり、動かすと戻る — *Type for a while without touching the mouse, then move it → The title bar fades while typing and comes back when the pointer moves*
- [x] **2.8** 何字か打って窓を閉じ、また起動する → 打った字が戻り、入力点はその後ろ — *Type a few letters, close the window and start it again → The letters come back and the input point is after them*
- [x] **2.9** 設定の「このノート」で「消せないモード」をオンにし、Backspace・Delete・← → どれも効かず、打つのは続けられる — *Settings, This notebook: turn on "no erasing", then press `Backspace`, `Delete` and `Left` → None of them does anything, and typing still goes on*

## 3. 上限

- [ ] **3.1** 「このノート」で字数の上限を 10 にして 12 字打つ → 10 字目で帯が上限を知らせ、鈴が 1 回鳴る。画面は変わらず、11・12 字目も入る — *Settings, This notebook: set the letter limit to 10, then type 12 letters → At the 10th the band says the limit is reached, a small bell rings once, and the screen stays as it is; the 11th and 12th go in*
- [x] **3.2** 時間の上限を 1 分、時計は最初の 1 字から。待ってから打つ → 最初の字までは時計が動かず、その 1 分後に帯が知らせる — *Set a time limit of 1 minute, with the clock starting at the first letter; wait, then type → The clock does not run until the first letter, and the band tells of the limit a minute after it*
- [x] **3.3** 上限を過ぎてから Esc、Enter → 続きを書ける。上限は「書く」ページのとおりに上がるか外れる — *Past a limit, press `Esc`, then `Enter` → The writing goes on, and the limit is raised or lifted as the Writing page says*

## 4. 清書

- [~] **4.1** 2 段落書いて Esc → 20×20 の原稿用紙・縦書き・明朝の清書。柱・魚尾・ページの番号 — *Write two paragraphs and press `Esc` → The clean copy shows them on 20×20 manuscript paper written vertically in mincho, with the column, the fishtail and the page number*
- [~] **4.2** 「PCー」を書き、道具の列で方眼・縦書き → 括弧と長音は縦の形、P と C は 1 マスに 1 字 — *Write `「PCー」`, then pick graph paper, vertical, in the tools row → The brackets and the long mark take their upright forms, and `P` and `C` go one to a square*
- [~] **4.3** 「今」「「」「A」で始まる段落を書く → 字下げは「今」の段落だけ — *Write paragraphs beginning `今`, `「` and `A` → Only the one beginning `今` is indented by one square*
- [~] **4.4** 行の頭に「。」が来るように書く → 前の行の最後のマスに入るか、「このノート」のとおりマスの外にぶら下がる。「。」」は 1 マス — *Write a line that fills the paper's line so that `。` falls at the head of the next → `。` goes in the last square of the line before, or hangs below the paper as the This notebook page says; `。」` share one square*
- [~] **4.5** `｜漢字《かんじ》` と `《《強調》》` → 「漢字」の横に小さくルビ、「強調」に傍点。記号は出ない — *Write `｜漢字《かんじ》` and `《《強調》》` → The reading sits small beside `漢字`, and dots beside `強調`; none of the marks shows*
- [~] **4.6** 道具の列で紙・向き・マスを変える → そのたびにすぐ描き直す — *Change the paper, the direction and the squares in the tools row → The clean copy is drawn again at once in each*
- [x] **4.7** Ctrl+C してメモ帳に貼る → 字下げした段落の頭は全角の空白。道具の列にコピーした字数 — *Press `Ctrl+C` and paste into Notepad → The text comes with a full-width space at the head of each indented paragraph, and the tools row says how many letters were copied*
- [x] **4.8** 清書の字をクリック → 書く画面に戻り、入力点はその字 — *Click a letter in the clean copy → The writing comes back with the input point at that letter*
- [x] **4.9** 清書の上で字を打つ → 書く画面に戻り、その字が入力点に入る — *On the clean copy, type a letter → The writing comes back with that letter added at the input point*
- [x] **4.10** 速さを変えて数行書き、1 度 40 秒止まって Esc → 行の横に速さの線、止まった所に点。「書く」で消せる。コピーと書き出しには入らない — *Write a few lines at different speeds, stopping once for 40 seconds, and press `Esc` → Beside each line is a line as long as it was fast, and a dot where the hand stopped; Settings, Writing can turn them off, and they are not in a copy or an export*
- [x] **4.11** 篇を 2 つ書き、清書で ← と → → 1 篇ずつめくる（縦書きでは ← が新しい篇）。端では帯がもうないと言う — *Write two pieces, then on the clean copy press `Left` and `Right` → The pieces turn over one by one (vertically, `Left` is the newer one), and at the end the band says there is no more*

## 5. ノートと本棚

ノートのフォルダーは既定で `ドキュメント\fude\<ノート名>\`。

- [x] **5.1** 篇を書いて Esc し、`ドキュメント\fude\ノート\` を開く → 書き始めた時刻の `YYYY-MM-DD_HHMM.txt`（UTF-8・LF・1 段落 1 行） — *Write a piece, press `Esc`, and open `Documents\fude\ノート\` → A `YYYY-MM-DD_HHMM.txt` named by when the piece was begun holds it, in UTF-8 with LF, one paragraph a line*
- [x] **5.2** Ctrl+N → 篇をファイルに綴じ、聞かずにすぐ新しい篇 — *Press `Ctrl+N` → The piece is bound to its file and a new one begins at once, with nothing asked*
- [x] **5.3** Ctrl+O → 本棚。左にノート、右にそのノートの篇（新しい順、字数・時間・1 分の字数） — *Press `Ctrl+O` → The shelf shows the notebooks on the left and the notebook's pieces on the right, newest first, with their letters, time and letters a minute*
- [x] **5.4** 本棚で N、「日記」、Enter → `ドキュメント\fude\日記\` ができ、そのノートが選ばれる — *On the shelf, press `N`, type `日記` and `Enter` → `Documents\fude\日記\` is made and the new notebook is picked*
- [x] **5.5** ノートを選んで F2、名前を変えて Enter → フォルダーの名前も変わる。`\` を含む名前やもうある名前は理由とともに断る — *Pick a notebook, press `F2`, rename it, and `Enter` → The folder is renamed with it; a name with `\` or one already there is refused with the reason*
- [x] **5.6** 篇を選んで Enter → その篇の清書が開く — *Pick a piece and press `Enter` → Its clean copy opens*
- [x] **5.7** 自分の .txt をノートのフォルダーに置いて本棚を開く → 篇として並び、開ける — *Put a `.txt` of your own in the notebook's folder and open the shelf → It is listed as a piece and opens as one*
- [x] **5.8** 清書で Ctrl+S → すぐ篇のファイルに書く — *On the clean copy, press `Ctrl+S` → The piece's file is written at once*

## 6. お題と記録

- [x] **6.1** 「このノート」でお題を 3 つ足し「一覧から順に」、Ctrl+N を 3 回 → 新しい篇ごとに次のお題が入力点の上に薄く、打ち始めるとさらに薄く。お題はノートの `fude.toml` に — *Settings, This notebook: add three topics, choose "in order", then press `Ctrl+N` three times → Each new piece shows the next topic faintly above the input point, fainter still once typing starts; the topics are in the notebook's `fude.toml`*
- [x] **6.2** 「くじ引き」、次に「出さない」にして新しい篇 → お題がくじで選ばれ、次は出ない — *Choose "by lot", then "none", and start new pieces → The topic is drawn at random, and then none shows*
- [x] **6.3** 2 日続けて書き、本棚を開く → 上に今日の字数・続けた日数・1 分の字数（この 7 日）。清書の下に今日の字数と続けた日数 — *Write on two days running and open the shelf → Above it: today's letters, the days in a row and the letters a minute over the last 7 days; under the clean copy, today's letters and the days*

## 7. 音

- [ ] **7.1** 「書く」でタイプライターを選び、打つ・消す・Enter・矢印 → 字と消すたびに音、Enter は重い音、矢印は鳴らない。選んだときに 1 回鳴る — *Settings, Writing: pick the typewriter, then type, erase, press `Enter` and the arrows → A key sound for each letter and each erasing, a heavier one for `Enter`, and none for the arrows; picking it plays it once*
- [ ] **7.2** 鉛筆を選んで速く打つ → 柔らかい音が 1 字ずつ。続けて打っても 1 つの音にまとまらない — *Pick the pencil, then type fast → A softer sound for each letter, and a run of keys does not run into one sound*
- [ ] **7.3** 音の大きさを 0 にして戻す → 鳴らなくなり、また鳴る — *Turn the volume down to 0, then up → The sounds go quiet and come back*
- [ ] **7.4** 最後の音から 1 分おく → 音の出口を閉じる（音量ミキサーから fude が消える） — *Leave the window a minute after the last sound → The sound device is let go (the volume mixer no longer lists fude)*

## 8. 設定

- [x] **8.1** Ctrl+, → 一般・テーマ・書く・このノート・キー・詳細・このアプリについて。ほかの uchmk のアプリと同じテーマと言語 — *Press `Ctrl+,` → The settings open on General, Theme, Writing, This notebook, Keys, Advanced and About, in the theme and the language the other uchmk apps use*
- [x] **8.2** 「書く」で字の大きさ・打つ所・帯に出すものを変える → すぐ効き、`config.toml` に書く — *On Writing, change the letter size, the input point and what the band shows → Each takes effect at once and is written to `config.toml`*
- [x] **8.3** 「このノート」で紙と上限を変える → ノートの `fude.toml` の `[note]` に書き、ほかのノートは自分の値のまま — *On This notebook, change the paper and the limits → They are written to `[note]` in the notebook's `fude.toml`, and another notebook keeps its own*
- [x] **8.4** ほかの uchmk のアプリでテーマを変える → fude も起動し直さずに従う — *Change the theme in another uchmk app → fude follows it without a restart*

## 9. キー・キー一覧・パレット

- [x] **9.1** 書く画面で F1、清書でも F1 → keymap のキー、書く・清書・本棚・固定のキーの順。清書の上で閉じると清書へ戻る — *Press `F1` on the writing, then on the clean copy → The key list shows the keymap's keys, then the writing's, the clean copy's, the shelf's and the fixed ones; closed on the clean copy, it goes back there*
- [x] **9.2** Ctrl+Shift+P で「設定」と打つ → 設定・書くのページ・このノートのページが並び、それぞれそのページを開く — *Press `Ctrl+Shift+P` and type `設定` → The palette lists the settings, the writing's page and this notebook's page; each opens the settings on its page*
- [x] **9.3** `keymap.toml` に `on = "<F2>"`・`run = "help"` の `[[app.prepend_keymap]]` を書いて F2 → 起動し直さずにキー一覧が開く。書き間違いは通知に出る — *Write `[[app.prepend_keymap]]` with `on = "<F2>"` and `run = "help"` in `keymap.toml`, then press `F2` → The key list opens without a restart; a misspelt command is told of in a notification*
- [x] **9.4** Ctrl+=・Ctrl+-・Ctrl+0 → 大きく・小さく・元に戻る — *Press `Ctrl+=`, `Ctrl+-` and `Ctrl+0` → Everything grows, shrinks and comes back to its size*
- [x] **9.5** Ctrl+Q → 篇をファイルに綴じてから閉じる — *Press `Ctrl+Q` → fude closes, and the piece is bound to its file first*

## 10. 書き出しと印刷

- [~] **10.1** 清書で Ctrl+E、Enter → ノートのフォルダーの `export\<篇>.pdf` に書いてフォルダーを開く。A4・明朝で、画面の清書と同じ — *On the clean copy, press `Ctrl+E`, then `Enter` → `export\<piece>.pdf` is written in the notebook's folder and the folder opens; the PDF shows the clean copy on A4 in mincho, as the window shows it*
- [x] **10.2** パレットで「書き出し: PNG（ノート 1 冊）」、次に短くした篇を同じく → 篇ごとにページの PNG。前の長い篇の余ったページは残らない — *Pick "Export: PNG (the whole notebook)" in the palette, then export a shorter copy the same way → A PNG is written to each page of every piece; no page of the longer copy is left behind*
- [ ] **10.3** Ctrl+P、Enter → 篇が印刷に回り、道具の列がそう言う — *Press `Ctrl+P`, then `Enter` → The piece goes to the printer, and the tools row says so*
