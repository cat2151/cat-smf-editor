# ADR 0025: 開いたばかりのTONE3000は、曲の前に読み込みを待ち、無音を通しておく

- 状態: 採用。待ちの上限3秒・無音4096 frame・判定のしきい値は仮決め
- 決定者: user（offline 描画で drum が頭から鳴らない症状と、起動直後の1回目の play だけ最初の drum がおかしい症状の報告。「曲の前に無音を chain 全体に通して処理させる」方式の指示）。読み込み完了の判定方法・待つ場所・却下した案はAIの設計
- 対象: TONE3000 CLAP、ID `com.tone3000.plugin`（track の effect として使うもの）
- 関連: [0017](0017-drum-sequencer.md)、[0022](0022-parallel-part-rendering.md)

## 症状と原因

- `--render-wav` で書き出した drum track（sforzando の kit ＋ effect の TONE3000）が、先頭5〜7秒は無音だった（user 報告）。
  TONE3000 は開いた後、壁時計で約100ms のあいだ model を裏で読み、その間は何も出さない。offline は実時間の約40倍で描くので、その100ms に曲の数秒が過ぎる。
  effect を外すと、待たずに描いても最初の note から鳴った。sforzando は開いた直後から鳴る。
- 読み込みの後も、TONE3000 が最初に処理する約2048 frame の音は小さい（最初の打撃の peak が 0.11、2回目の play では 0.48）。
  起動直後の1回目の play は、開いたばかりの instance に曲をすぐ送るのでこれが出る（user 報告の「1回目だけおかしい」）。

## 決定

- 新しく開いた（または reload した）track の chain に TONE3000 があれば、曲の最初の block の前に次をする。
  1. TONE3000 を含む effect chain に試験音を通し、音が返るまで待つ。読み込みの完了を host へ知らせる手段は見つからなかったので、出力で判定する
  2. 音が返ったら、track 全体（instrument と effect）に無音を通す。試験音の余韻が消えるまで、少なくとも4096 frame
- offline（`--render-wav`・recent の先読み・song challenge）は描画の worker thread で、job の頭でする。
- 実時間の再生は、stream を始める直前に UI thread でする。使い回した instance ではしない（済んでいる）。
  起動の経路では開いてから stream まで約1.7秒あるので、待ちはほぼ無く、増える時間は約30ms。

## 却下した案

- 描画の前に決まった時間だけ待つ: 必要な時間は model の大きさと disk の速さで変わり、足りなければ同じ症状が黙って出る。
- 無音を処理するだけ（待たない）: 読み込みは壁時計で進み、処理した frame 数では進まない。読み込みが終わる前に通した無音は、1回目の小さい音も直さない。
- sforzando の sample 読み込みを待つ（note を当てて音が返るまで待つ）: sforzando は開いた直後から鳴っており、症状を変えなかった。
- すべての effect で試験音を待つ: 試験音に無音を返す effect（gate など）があると、毎回上限まで待つ。
