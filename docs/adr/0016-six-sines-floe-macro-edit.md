# ADR 0016: Six SinesとFloeのMacroをMacro editの対象に加える

- 状態: 採用
- 決定者: user（editorではCC1割り当て済みのMacroも対象とし、playerの除外規則は維持すること）
- 関連: [0014](0014-surge-macro-edit.md)、[0015](0015-vaporizer2-macro-edit.md)、[player 0039](../../../cat-plugin-player/docs/adr/0039-parameter-macro-sweep.md)

## 決定

Six Sines CLAP（`org.baconpaul.six-sines`）のMacro 1～6と、Floe CLAP（`com.floe-audio.floe`）のMacro 1～4を、Surge XT・Vaporizer2と同じMacro edit・Variation・Macro variation curveの対象にする。保存meta・試聴・確定の経路は0014をそのまま使う。

使用判定・名前・polarity・host parameterとの対応・値域は、playerと共有する`cat-plugin-player/shared/instrument_macros.rs`の`Instrument`にまとめ、editorは`Instrument::of`だけを入口にする。3音源の判定方法はplayer ADR 0039に書いた。editorとplayerで判定を二重に持たないため。

- 動かすparameter: Six Sinesは`Macro N Level`（値域-1～1）、Floeは`Macro N`（0～1）。
- Six Sinesは常にbipolar（中央0）で表示する。Surge XTと違い、API値もそのまま-1～1である。

## CC1による対象除外の撤回

当初はSMFのCC1 eventとMacro editのcurveが同じMacroを取り合うと考え、playerのCC1欄との競合を避ける`Instrument::movable`をeditorにも適用した。その結果、Floeの工場出荷時のMacro 1は使用中でも編集できず、Macro 1だけを使うpatchではMacro editを開けなかった。

editorの再生経路を確認すると、SMFから音源へ送るのはNote On / Offで、CC1は送っていない。このためeditorにもCC1との競合回避が必要という前提を撤回し、userは使用中のMacroをそのまま編集対象にする方針を選んだ。使用判定・名前・値域は引き続き共有し、editorは`Instrument::routed`を使う。playerはCC1の自動操作があるため`Instrument::movable`による除外を維持する。editorが将来CC1の再生に対応する場合は、その機能でMacroとの優先関係を決める。

## 値の持ち方

Macro editの行の値（音声callbackが読む値）は、API値で持つ。画面の操作位置0～1との変換は、そのcurveの値域（`shared/parameter_domain.rs`）を通す。値域が0～1の音源だけなら位置とAPI値が同じになるが、Six Sinesの-1～1では位置をそのまま送ると値がずれる。

初期値は、保存済みのcurveがあればそのAPI値、無ければstateのMacro位置をcurveの値域へ写した値にする。curveの`minimum`/`maximum`にはhost parameterの値域を入れ、`Instrument::range()`と一致するparameterだけを選ぶ。保存metaの読込も、値域が`Instrument::range()`と一致するcurveだけを受け付ける。Macro variation curveはsnapshotのAPI値を線形に混ぜるだけなので、値域が-1～1でも変更は要らない。

## 実機での読み戻しの罠

- Six Sinesの`current`は平滑化された値を返す。数blockの送信では送った値に届かない。
- Floeは、stateを読み込んだ直後しばらく実時間が経つまでparameterのeventを捨てる。送った値は、main threadのpumpの後で`current`に出る。
- Surge XTは、数block送ると送信が`-6`（stale revision。metadata rescan）で拒否されることがある。実機testは`-6`のときだけ`prepare_automation`を取り直す。再生中の扱いは既存のmetadata変更通知での取り直しに任せている（未確認）。

## 却下案

- Six Sines・Floe用の判定をeditor側に持つ：playerのMacro欄と判定が二重になり、片方だけ直す不整合が起きるため採らない。
- 値域0～1を前提にしたまま、Six Sinesだけ送る直前に-1～1へ写す：保存meta・Variation・curveが位置とAPI値を混同したままになり、0014で区別したmacro値・操作位置・API値の関係が崩れるため採らない。

## 制限

- コマンドパレットの「… Macro edit」は、patchのMacro使用を見ずに対象音源のlayerごとに出る。使用中のMacroがないpatchでも候補に出て、選ぶと「This patch has no routed macros」になる。
- 読込直後に実時間を待たないoffline render（Ctrl+Pの試聴の書き出しなど）では、FloeのMacroの値が反映されないことがある（未確認）。
