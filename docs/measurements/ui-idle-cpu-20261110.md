# 何も鳴らしていない間の CPU：UI の描画を止めた後の再計測

前の計測は [audio-idle-spin-20261110.md](audio-idle-spin-20261110.md)（音声の stream を休ませた後も、何も鳴らしていない間に 1 本の thread が 0.82〜0.98 core を使い続けた）。その 1 本は UI thread で、何もしていない間も 58〜60 fps で描画し続けていた。

## 直したもの

何もしていない間の毎 frame に描画を頼んでいた 3 か所と、出力の解析表示。

| 原因 | 直し方 |
|---|---|
| 子 viewport へ `ViewportCommand::Visible` を毎 frame 送り、egui がそのたびに描画を予約する | 値が変わったとき・前の pass に出していなかった窓へだけ送る（`src/view/presentation/windows.rs` の `send_visible`） |
| Virtual MIDI keyboard の session がある間、無条件に 8ms 後の描画を頼む | 時間で進む仕事（押しているキー、phrase loop の Capturing・Looping、録音、未確定の Capture など）がある間だけ頼む（`Session::ticking`） |
| 音声の voice がある間、32ms 後の描画を頼む（曲が終わっても voice は残る） | 開いている stream が全部休んでいて plugin の main thread の仕事も無い間（`Audition::idle`）は 500ms。出力の解析表示はその間 50ms の描画を頼まない |

## 結論

条件: release build を、user の設定の複製（temp の `LOCALAPPDATA`）と曲の複製で、user の session（選んでいる track・開いている窓。解析表示と help 窓も開いている）のまま起動。起動時の自動再生の後、曲の stream と keyboard の stream の両方に `[audio-rest]` が出るのを待ち、1 秒後から約 6 秒、本体の process と thread ごとの CPU 時間を取る。その後 Space で再生し、6 秒の frame を見る。frame の数と描画の依頼元は `CAT_SMF_FRAME_TRACE` の frame trace から出す。手順は `python scripts/measure_frames.py saved idle` が全部行う。本体 CPU は 12 論理コアを 100% とする。

| 条件 | 目標 | 直す前（2 回） | 直した後（3 回） | 合否 |
|---|---|---|---|---|
| 何も鳴らしていない間に 0.8 core を超える thread | 0 本 | 1 / 1 本（UI thread 0.81 / 0.95 core） | **0 / 0 / 0 本**（UI thread 0.25 / 0.23 / 0.23 core） | 合格 |
| 何も鳴らしていない間の本体 CPU | 4% 以下 | 8.6 / 9.3 % | **2.7 / 2.4 / 2.1 %** | 合格 |
| 再生中の描画回数（frames/s） | 55 以上 | 58.8 / 58.4 | **57.6 / 58.7 / 58.7** | 合格 |
| 再生中に再生位置の表示が動いた回数/秒 | 55 以上 | 58.8 / 58.4 | **57.6 / 58.7 / 58.7** | 合格 |
| 再生中に出力の解析表示が描画を頼んでいる（`src\view\analysis.rs`） | 出ている | - | **57.4 / 58.6 / 58.6 回/秒** | 合格 |

- 3 回とも、両 stream の `[audio-rest]` は待ち時間内（90 秒）に出た。曲の複製元の hash は前後で一致し、終了後に本体の process は残っていない。

## 何もしていない間の描画

| 回 | frames/s | UI thread CPU | 描画の依頼元（回/秒） |
|---|---|---|---|
| 1 | 11.6 | 23.8 % | `layout\runtime.rs:30`・`help\window.rs:140`・`mod.rs:382` 各 11.6、`analysis.rs:162/163` 2.7 |
| 2 | 12.1 | 23.0 % | 同 3 か所 各 12.1、`analysis.rs:162/163` 2.3 |
| 3 | 11.2 | 22.4 % | 同 3 か所 各 11.2、`analysis.rs:162/163` 2.5 |

- 直す前は 59.1 / 60.0 fps で、1 frame の CPU が vsync の 16.7ms をほぼ使い切っていた。
- 残る依頼は `layout.toml` の見張り（500ms）、help 窓（100ms）、休んでいる間の pump（500ms）。3 か所とも毎 frame 出ているので、どれか 1 つ（最短は help 窓の 100ms）が frame を起こし、他は同じ frame で次を頼み直しているとみる。
- 何もしていない間にも解析表示の依頼が 2〜3 回/秒残る。`[未検証] plugin の main thread の仕事待ちか、mailbox の try_lock が取れずに「起きている」と読んだ frame とみる（原因は調べていない）`
