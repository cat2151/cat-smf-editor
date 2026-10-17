# ADR 0005: テンポカーブは format 1 の conductor track に保存し、tempo event をそこから作り直す

- 状態: 採用
- 関連: [0004](0004-master-effects-in-conductor-track.md)

## 決定

テンポカーブは SMF format 1 の先頭track（conductor track）の tick 0 に、
Sequencer Specific Meta Event (`FF 7F`) として1個保存する。

payload は `\x7dcat-smf-editor\0tempo-curve\0` と UTF-8 JSON の連結。

```json
{"version":1,"points":[{"tick":0,"bpm":120.0}],"controls":[]}
```

- カーブはテンポポイント `P0..Pn`（tick 昇順・重複なし）と、区間 i ごとのコントロールポイント `Ci` 1個の2次ベジェ。
  `controls.len() == points.len() - 1`。`Ci.tick` は区間 `[Pi.tick, Pi+1.tick]` の内側に限る。
- 最初の点より前は最初の点の BPM、最後の点より後は最後の点の BPM で一定。
- metaがある曲では、conductor の tempo event（`FF 51`）をすべてカーブから作り直す。
  書く tick は {0} ∪ 全trackの Note On / Note Off の tick。直前と同じ µs は書かない。
- 作り直しは `App::edit_music` の最後で毎回行う。管理中のtrillがあれば秒間隔に合わせてノートを再配置し、その最終位置からtempo eventを生成する。他の編集でノートの tick が変わっても tempo event が追従する。trillの保存形式と、生成したtempoとの整合方法は[trillの管理と再配置](../trill.md)を参照。
- metaなしはカーブなし。最後の1点を消したら meta だけを除去し、その時点の tempo event は残す。
- 重複、先頭track以外、tick 0以外、format 1以外、未知version、点の順序違反、controls 数の不一致、
  SMF で表せない BPM（tempo 1〜0xFFFFFF µs の外・非有限）、壊れた JSON はエラー。黙って捨てない。
- 独自 fermata 機能は撤去し、テンポの変化はテンポカーブで扱う。併用のための排他制御や旧データ移行は持たない。

### 目安テンポ

カーブ画面では、conductor の既存 tempo event の各 tick から目安テンポを表示用に仮生成する。
開閉・スクロール・ズームでは曲・保存待ち内容・Undo履歴を変更しない。
カーブ・目安テンポの実編集、またはtrillの変更確定時に、Text meta (`FF 01`) をその編集とまとめて記録する。
カーブによるtempo eventの置換より前のBPM・順位を使う。初期化を含めUndo 1回で戻せる。
本文は英語の `Reference Tempo: 120 BPM; rank=2`。BPM は有限かつ SMF のテンポ範囲内、rank は正の整数。
同じ tick に複数の tempo event があれば最後の値を使う。tick 0 に tempo がなければ既定の 120 BPM も保存する。
目安 meta が1個でもあれば保存済みの値を使い、初期化しない。初期化済み・カーブ編集歴のフラグや旧データ移行は持たない。

- rank は初回の BPM の速い順。同値は同順位で、異なる値には 1・2・3 と欠番なく順位を割り当てる。
- 区間ごとの水平な破線を上下ドラッグして BPM だけ変更する。tick と初期 rank は保持する。
- 現在の目安 BPM 同士から同じ方式で順位を計算し、初期 rank と異なる線は橙色、一致する線はピンク。
  カーブの値は順位比較に使わない。ドラッグ中もプレビューし、離したときに1回保存する。
- カーブの点と重なる場所は点操作を優先する。クリックによるカーブ点の追加は従来どおり。
- 目安の編集は実際の tempo とカーブを変えない。目安はカーブの再生成・全削除後も残す。
- 重複 tick、不正な BPM・rank、format 1 の先頭 track 以外にある目安 meta はエラー。

## 理由

- tempo は曲全体に1本なので、master effect（ADR 0004）と同じく conductor に置く。保存場所と検査規則を揃えられる。
- 生成した tempo event だけを保存すると、カーブの形（どこが頂点で、どう曲がるか）を再起動後に編集し直せない。
- tempo event を Note On / Note Off の tick だけに置くのは、テンポが聴こえ方に効くのが発音と消音の位置だけだから。
- tick 0 を含めるのは、最初のノートまでの無音が SMF 既定の 120 BPM で鳴らないようにするため。
- 2次ベジェで `Ci.tick` を区間内に clamp すると、tick に対する x(s) が単調になり、tick → BPM が一意に決まる。
  縦横どちらの曲がりも1個の制御点で表せる。
- 以前の独自 fermata は「次の Note On で元のテンポに戻す」tempo event を書いていた。カーブは tempo event を全部作り直すため衝突する。カーブがあれば独自 fermata は不要と判断し、排他制御で併存させる方式をやめて機能を撤去した。

## 却下した案

- **生成した tempo event だけを書き、カーブは保存しない。** 再起動後にカーブを編集できない。
- **一定 tick 間隔で tempo event を書く。** 間隔の設定項目が要り、ノートの無い所にも event が増える。
- **3次ベジェ・区間ごとの補間種別の切り替え。** 2次で上下左右の曲がりを表せる。設定と UI が増える。
- **独自 fermata をカーブの上に重ねて適用する（撤去前の検討）。** 2つの tempo 源の優先順位という新しい規則が要るため却下した。現在は独自 fermata 自体を撤去している。
- **format 0 / 2 への対応。** ADR 0004 と同じく、検証できる運用データが format 1 だけ。

## 結果

カーブがある曲では、conductor の tempo event は手で編集しても次の編集で上書きされる。
元の tempo は原本 SMF に残る（保存は従来どおり保存コピーへ行い、原本を上書きしない）。
format 0/2 でカーブが必要な運用データが現れたら、このADRを見直す。

## 壊れたら気づく場所

| テスト | 落ちたら |
|---|---|
| `a_straight_segment_writes_linear_tempos_where_notes_start_or_end` | tempo event の tick か値がカーブとずれた |
| `a_flat_120_bpm_curve_times_notes_like_a_plain_500000_tempo` | カーブを付けるとノートの実時間が変わった |
| `metadata_round_trips_and_keeps_notes_names_and_master_effects` | 保存でカーブ・ノート・他 meta が壊れた |
| `malformed_metadata_is_an_error` | 不正な保存場所・schema を受け入れた |
| `editing_notes_regenerates_tempo_events_at_the_new_note_ticks` | ノートを動かしても tempo event が追従しない |
