# ADR 0004: master effect は format 1 の conductor track に保存する

- 状態: 採用
- 関連: [0003](0003-shared-host-player-selection.md) / [player ADR 0030](../../../cat-plugin-player/docs/adr/0030-master-effects-selection.md)

## 決定

master effect は SMF format 1 の先頭track（conductor track）の tick 0 に、
Sequencer Specific Meta Event (`FF 7F`) として1個保存する。
先頭trackに演奏イベントがあっても、新しいconductor trackの追加やtrackの移動はしない。

payload は `\x7dcat-plugin-player\0master-patch-references\0` と UTF-8 JSON の連結。

```json
{"version":1,"layers":[],"effects":[{"patch":{…},"bypass":true}]}
```

- `effects` はtrack effectと同じ`PatchReference`（plugin identity・元presetのpath/value・表示名）を持つ。配列順が信号順。state本体は保存せず、終了時の永続cacheとplayer生成要求から復元する。
- masterにinstrumentは持たせない。trackの `instrument-layers` metaとはprefixと型を分ける。
- `bypass`はeffectごと（[ADR 0007](0007-effect-chain-on-uapmd-graph.md)）。metaなしはeffectなし。全effectを削除したらmetaを除去する。
- 重複、先頭track以外、tick 0以外、未知version、破損・過大payloadはエラー。黙ってdryにしない。
- 更新するのはmaster metaだけ。ノート・tempo・track chain・未知metaを保持し、従来の保存コピーへ保存する。原本を上書きしない。
- 信号順は `各layer instrument → layer gain → track内合成 → track effect → track gain → 全track合成 → master effect → Master auto gain → 出力`。
- `m` はmaster欄を選択してmixerを開く。続けて `x`（`mx`）でmaster effectのoverlayを開く。
  `l`／右で先頭track、`h`／左でmasterへ戻れる。track選択後の `x` はtrack effect。
  `j/k` はtrack音量だけに適用し、master欄では音量を変更しない。

## 理由

- masterは曲全体へ1回適用する設定なので、各演奏trackに複製せずconductorに置く。
- 既存chainはinstrument必須。ダミーinstrumentを保存すると、復元時にmasterとtrackの意味が混ざる。
- 現在userが検証できる実データはformat 1。format 0/2へのmaster対応はYAGNIとして実装しない。
  既存のformat 0/2の読込・再生は削除しないが、masterの選択・保存対象には含めない。

## 却下した案

- **master専用trackを追加する。** track番号と現在の選択位置を変える必要がある。
- **既存track chainにmasterを混ぜる。** conductorに音色がある場合のtrack effectと区別できない。
- **format 0を1へ変換し、format 2に共通masterを定義する。** 検証できる運用データが無く、不要な変換と解釈を増やす。

## 結果

resident playerへmasterの対象欄を送り、選択したpresetのstateと復元用参照を受け取る。
editorがmaster専用metaだけを更新し、現在の演奏位置から再開する。
cacheからの復元ではbrowserを表示しない。共通の保存・生成・cache契約は[ADR 0006](0006-instrument-layers-and-patch-state-cache.md)を参照する。
layer合算と共通effectの順序・回数はrendererテスト、masterの参照保存・配置保護と反映はSMF/IPCテストで確認する。実pluginの複数layer発音・停止と音声出力を検証済み。人間による音色・操作感の合格とは区別する。
format 0/2のmasterが必要な運用データが現れたら、このADRを見直す。

## 壊れたら気づく場所

| テスト | 落ちたら |
|---|---|
| `master_reference_smf_roundtrip_preserves_preset_value_other_events_and_removes_empty_meta` | conductorへの保存で既存イベントやtrack chainが壊れた |
| `master_reference_invalid_placement_duplicates_version_and_legacy_meta_never_overwrite` | 不正な保存場所・schemaを受け入れた |
| `master_store_updates_only_conductor_metadata_and_preserves_track_settings` | master帰還時にtrack設定を変更した、または古い結果を取り込んだ |
| `mx_selects_master_without_changing_track_and_navigation_keeps_track_gains_separate` | master選択とtrack音量操作が混ざった |
| `master_exchange_uses_own_metadata_and_rejects_preview_track_changes` | playerへmaster対象を伝えられない、またはtrack変更を受け入れた |
