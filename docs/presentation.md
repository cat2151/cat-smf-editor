# 操作画面と補助窓の表示

`src/view/presentation.rs` が、command の入力所有権と画面の優先順位を対応付ける。
範囲選択menuだけを前面にしたり、Virtual MIDI keyboardだけを隠したりする個別判定は追加しない。

## 表示規則

- `Screen::presentation` は全Screenの表示方針を列挙し、`allows_companions`と画面内の描画層を導く。通常の作業画面と範囲選択中（menuを開く前）は補助窓を併用でき、menu・入力・確認中は操作を載せたviewportを優先する。
- パレットは起点のviewport、独立したcommand入力はroot、Scene/Songの入力はそのdialogのviewport、再生menuは開いたviewportを優先する。
- egui modal、context menu、menu button、popupは共通の生成経路で表示先を登録する。別窓内の操作も対象とする。popup表示中は背後のcommand処理と鍵盤のpad入力を止め、Escを親画面へ漏らさない。同じ窓群の他の窓も退避し、rootの描画末尾でも全登録窓へ最終的な表示判定を送る。
- 退避は非表示で行う。window、session、位置、入力内容を破棄せず、非表示のcallbackは実行しない。閉じる操作・queued inputも通さない。復帰時は元の操作先へfocusを戻し、Windowsが復帰した補助窓をactivateしても入力先を奪わせない。
- ヘルプは現在の操作中に明示的に呼ばれた場合に表示できる。別の操作へ移った際にはこの許可を引き継がない。
- Overviewは補助窓を退避するが、画面内では作業画面の層を保ち、その右クリックmenuを覆わない。
- rootの浮動操作画面はForegroundに置き、入力所有者を同じ層内でも手前にする。背景の操作は無効にする。ドラム・パラメータカーブ内の入力は、その入力を載せる画面で操作できる。

## 画面を追加する場合

1. `Screen`へ追加し、入力所有権と`presentation`の方針を決める。網羅的matchのため表示規則の追加漏れはcompile errorになる。
2. rootの浮動窓は`presentation::window`、浮動入力は`presentation::area`を使い、Screenと実際の安定IDを渡す。egui modalは`presentation::modal`を使う。context menu・menu button・popupは同moduleの共通関数を使う。
3. ネイティブ窓は`presentation::Native::show`を使う。独自の`show_viewport_immediate`や別窓固有のmodal退避判定を作らない。builderのvisibleは画面自身の開閉条件だけに使う。
4. `palette/tests/windows.rs`へ画面・窓のfixtureを追加する。presentationの共通テストはそのfixtureを使い、全root操作画面と全別窓種別の退避を確認する。
5. root描画の`prepare`・`refresh`・`finish`を維持する。別窓の表示後に開いた操作も、`finish`で最終判定へ反映される。

鍵盤は表示と音声進行を分けている。退避時には押下中のpadを解放し、入力を止める一方、録音clock・capture・phraseは進行する。再表示で録音sessionを作り直さない。
保存形式・復元形式は変更していない。

## 検証

`cargo test --bin cat-smf-editor presentation`、`cargo test --bin cat-smf-editor suspension`と全体テストで、画面所有権、別窓の表示遷移、遅れて開く別窓内modal、非表示窓への入力、背景click、録音の保持を確認する。
