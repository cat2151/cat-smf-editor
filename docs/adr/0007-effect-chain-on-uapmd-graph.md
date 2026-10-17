# ADR 0007: effect chainはUAPMDのlinear graphで処理し、bypassはeffectごとに持つ

- 状態: 採用
- 関連: [0004: master effect](0004-master-effects-in-conductor-track.md)、[0006: instrument layer](0006-instrument-layers-and-patch-state-cache.md)

## 決定

- 共通hostの`uh_chain_create`は、effect列をUAPMDの`uapmd_graph::AudioPluginGraph`（linear）へ並べて処理する。
  instrumentは従来どおりchainの先頭stageとして処理し、その出力をgraphへ渡す。
- graphの共有contextはstereoのmain busから始め、各effectのbus構成で広げる。UAPMDのsequencerがtrack contextに行う統合と同じ規則にする。
- bypassはeffectごとに持ち、`uh_chain_set_bypassed`でgraph nodeのbypassを切り替える。再生中に呼べ、chainを開き直さない。
- chain全体の`bypass`は、editorの保存形式（`instrument-layers`・`master-patch-references`）と`master_effects::Settings`から除く。
  保存形式の`effects`は`{"patch": PatchReference, "bypass": bool}`の配列で、配列順が信号順。
  playerの試聴用chain全体bypass（`chain_settings::Settings.bypass`）は残し、全effectのbypassとして適用する。
- editorはbypassだけが変わった編集では演奏を止めずに稼働中のchainへ反映する。それ以外の変更は従来どおり開き直す。
- track effectとmaster effectの編集は専用overlayにまとめ、信号の流れを縦に描く。mixerとtrackヘッダーはoverlayの入口だけを持つ。

## 理由

- 自前の直列chainはbypassがchain全体に1つで、effect単位の比較ができなかった。UAPMDのgraphはnode単位のbypassと、bypass時の入出力の素通しを持っている。
- bypassでchainを開き直すとplugin生成を待つうえ、演奏が途切れて聴き比べにならない。
- effectの追加・削除・並べ替え・bypassを1つの画面に集めると、mixerやheaderに同じ操作を重複させずに済む。

## 却下した案

- 自前chainにeffectごとのbypass配列を足す: 変更は小さいが、UAPMDへ移行する方針（ADR 0006）と逆行する。
- bypassしたeffectを外してchainを作り直す: C++を変えずに済むが、切り替えのたびにpluginを開き直す。
- 汎用DAG（`AudioPluginFullDAGraph`）を使う: 現在の信号は直列だけで、分岐・合流の需要がない。

## 結果

- 並べ替えは、graph上の移動ではなくchainの開き直しで反映する。
- 同じpluginを同じchainへ2つ置けない制約（`place`の重複検査）は変えていない。
