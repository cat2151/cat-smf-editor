# 横timelineのadapterと共通操作テスト

## 接続の単位

横方向のminimap・A-B repeat・zoomは `src/view/timeline_controls.rs` を入口にする。`screens!(...)` の一つの宣言から `Screen` と `Screen::ALL` を生成し、同じ一覧をproductionと実UIテストが使う。縦の `track_map` とevent listの `event_repeat` は別の部品である。

`Viewport` はPiano roll・五線譜・Track Overview・Song/Sceneで使う座標状態、`Horizontal` はTempo/Parameter curveで使う横状態を持つ。Tempo側のviewportは縦axisを担当するadapterとして `Horizontal` を内包する。共通の `zoom` と `Zoom` が倍率・anchor・範囲fit・左右ボタン・wheelを扱う。範囲drag中は倍率を変えず、選択範囲を実contentの上にも半透明で描き、minimapには現在の枠に重ねてrelease後の枠を描き、releaseでfitする。

画面の1フレームでは次の順に接続する。

1. `viewport.begin(Screen::...)` で実画面を登録する。別の操作対象に切り替える際は未完のgestureとpending移動を解放する。
2. 実contentのaxisと全対象の終端tickから、一つの `Binding` を作る。tick比例はpixels-per-tickの `f32`、五線譜は `staff::spacing::Spacing`、Song/Sceneはセルのaxisを渡す。`Binding` のviewportは前回の実ScrollArea出力に基づき、固定ラベル・鍵盤・調号・inset・scrollbarを除く。
3. 同じbindingをminimapとrepeatへ渡す。minimapの全対象tickと、repeatのscrolled content座標を取り違えない。Piano roll系の `minimap_on` とTempo系の `minimap` は背景描画・pending適用のadapterであり、gestureを実装しない。
4. followを手動minimap移動で解除し、そのフレームのrevealよりも手動移動を優先する。seek tickが返るのはclickだけである。dragでは文書・undo・repeat・再生scopeを変更しない。
5. 実ScrollArea出力を `Viewport::finish` または `Horizontal::finish` へ戻す。セルadapterも同じ実offsetとcell strideを次フレームへ返す。repeat専用ScrollAreaは作らない。
6. 閉じる・非表示にする操作で `suspend`、または画面stateの破棄を行い、古いgesture/pendingを次回の画面へ持ち越さない。

たとえばtick比例の画面では、unzoomed layoutから横zoomを反映した `time_scale` を一度求める。

```rust
viewport.begin(Screen::PianoRoll);
let time_scale = overview.scale * viewport.zoom().x;
let timeline = viewport.binding(&time_scale, end_tick);
// minimapにはtimelineと全対象の背景描画を渡す。
// content直上のrowには同じtimelineを渡す。
let action = timeline.repeat(ui, row, range, repeat_state);
// domainのcallbackでactionを適用し、実ScrollArea出力でfinishする。
```

非等間隔axisでは `TickAxis::x` / `tick` と表示移動用の `x_at` / `tick_at` を対応させる。描画と入力の原点を揃え、glyphの余白やセルのlabel/gapはadapterで明示する。背景ノート描画は `piano_roll::minimap`、tempo背景は `timeline_minimap::tempos`、arrangement背景は各workspace adapterが持つ。

## 登録と意図した違い

停止中clickの副作用だけを `Screen::policy` の `Click`（`SeekPlaying` / `Cursor` / `Song` / `Scene`）で管理する。再生・カーソルへの適用は `timeline_domain` のadapterが担当し、共通gestureは再生modeを所有しない。再生中は既存transport/domainのseek規則を使う。

| Screen | axis | 部品 | 停止中click |
|---|---|---|---|
| Staff | 非等間隔spacing | minimap / repeat / zoom | 表示移動 |
| PianoRoll | tick比例 | 同上 | 表示移動 |
| Tempo | tick比例 | 同上 | 表示移動 |
| TrackOverview | tick比例、固定track名を除く | 同上 | 表示移動 |
| Parameter | tick比例、値axisを除く | 同上 | 表示・カーソル移動、auditionなし |
| Song | Song拍子に対応する等幅セル | 同上 | 表示移動とSong再生 |
| Scene | Scene自身の拍子に対応する等幅セル | minimap | 表示移動とScene再生 |

Songのrepeat callbackは `start_song` を使い、pattern候補のscopeへ切り替えない。Parameterはownerとvariationの縦axisを保全する。Sceneにはrepeat/zoomバーを追加しない。背景や縦axisの違いを横gestureの例外に広げない。

## 新画面を加えるとき

1. `screens!(...)` の登録宣言にIDを加え、click policyと実在する部品を宣言する。未分類を名前の推測や任意のopt-outで処理しない。追加の意図的な操作差にはuserの判断が必要である。
2. 実contentのaxis・範囲・ScrollArea出力を既存adapterと同じ入口に接続する。Views等の埋め込み経路は画面adapterを再利用する。
3. `timeline_controls/tests/fixtures.rs` のexhaustive matchに、実画面の起動・入力投入・描画観測を追加する。fallbackを置かない。新IDにfixtureがなければ、matchがコンパイルエラーになる。
4. 共通のdrag・zoom・click policy・repeat操作列を通す。期待tickは既知の音符、描画された小節やcurveの印から決め、adapterのaxis関数で期待値を再計算しない。Sceneのバー除外は実際の画面構成に対応する。
5. キーボード所有権、overlay、IME、複数pane、resize、縦操作との非干渉を、関連する既存画面テストでも確認する。

`Screen::ALL` は共通入口を使う登録の網羅性を検査する。新しい画面が入口を丸ごと迂回する実装まで自動発見するものではないため、利用箇所の検索とAGENTS.mdの規則も維持する。

## 検証コマンド

```text
cargo test --bin cat-smf-editor view::timeline_controls::tests -- --test-threads=1
cargo test --bin cat-smf-editor view:: -- --test-threads=1
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --release
```

共通テストはproductionの画面を開き、minimap枠内外と両端のdrag、release後の位置、follow解除、停止中clickのpolicy、再生中のscope/位置/repeat保持、左右click・正逆/左右buttonのrange zoom・wheel、range drag中の倍率維持・content上の範囲描画・minimapのrelease後の枠、描画された印からのrepeat、非表示・再表示時のdrag解放、無効化されたrepeat入力を確認する。既存のPiano roll、Tempo、Staff、Parameter、Overview、workspaceテストが、keyboard、scrollbar/page、非等間隔spacing、拍子変更、repeat scope、縦axis、overlay/IME等の契約を補う。

登録漏れの負例検査では、一時IDを `screens!(...)` とproduction policyへ追加しfixtureを用意せず、`cargo check --tests` がfixtureのexhaustive matchで失敗することを確認する。一時変更は原状へ戻してから必須チェックを行う。試行用backupとログはrepository外へ保存する。
