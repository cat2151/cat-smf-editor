# ADR 0010: MML・コード進行入力から現trackへのnote挿入

## 背景

cat-plugin-playerと同じ入力とカーソル試聴を使い、生成したSMFのnoteをeditorで検証する。変換器、入力欄、試聴対象の判定を複製すると、両アプリの結果がずれる。

## 決定

- playerの`shared/mml.rs`、`shared/mml_preview.rs`、`shared/mml_widget.rs`を正本とし、chord判定、SMF生成、時刻付き演奏への変換、カーソル位置の発音単位、入力overlayを共有する。音声出力・確定後の用途・保存はアプリ側に残す。
- `i`でmodal overlayを開く。入力・左右キーでカーソル位置の音符または和音を試聴する。入力が変わるたびに開いた時点の曲へ挿入し直して曲へ反映し、全viewを追従させる。発音noteにならない入力は開いた時点の曲を表示する。`Enter`は挿入できない入力ならerrorを出して開いたまま、`Esc`またはCancelは曲とカーソルを開いた時点へ戻して閉じ、Undo履歴を増やさない。`Enter`で確定したときだけUndo 1段を記録し、末尾の休符を含むMMLの終了tickへカーソルを進める。入力欄の操作中は他の編集・移動ショートカットへ入力を渡さない。既存のoverlay、menu、browserの操作を優先する。
- editorはoverlayを開く時点の現trackと現在tickを挿入先として固定する。A-B repeat演奏中は範囲・全track演奏・followを保ち、入力変更を既存のtimeline再開経路で現在の演奏tickから反映する（音声の再開による短い途切れは許容する）。この間はカーソル試聴で音声を置き換えず、確定時の1回演奏も行わない。確定・キャンセル後もrepeatを継続する。入力中のShift+Spaceは演奏とrepeatを止め、以後の入力変更・確定・キャンセルでは再開しない。それ以外の演奏はoverlayを開くときに停止する。生成SMFの絶対tickを曲のPPQへ四捨五入で換算して基準tickへ加算する。tempo指定は音符のtickを変えず、曲のtempoを優先する。SMPTE分解能への挿入は拒否する。
- 生成SMFからnote on/offのみを挿入する。tempo、program change、CC、meta、SysExは取り込まない。既存の音色・設定・note・後続tickを保全し、End of Trackだけ必要に応じて延長する。保存は既存の経路を使う。
- **検証用途のeditor固有仕様**として、`;`で複数track/channelになった場合も、全noteを現trackの同一channelへ変換する。channelは選択eventのchannel、既存noteのchannel、既存channel eventのchannelの順で採用し、metaのみ・空trackはMIDI channel 1とする。元channelの分離は保持しない。同音高の重複noteも統合・除去せず、入力由来のeventを保持する。
- playerは従来どおり確定したphraseをCustom sequenceとして使い、editorの挿入先・channel決定は共有コードへ持ち込まない。

## 保存形式と移行

SMF・設定・sessionの保存形式と復元経路は変更しない。editorの入力文はoverlayを開くたびに空から始め（入力は開いた位置へ挿入する一回分の用途のため）、既存SMFには通常のnote eventsとして保存する。playerのconfirmed MMLとCustom sequenceの復元も既存フィールドのままなので、既存データの移行は不要。

## 検証

共通カーソル試聴の既存テストを移し、MML/chord・複数trackの文脈とgateを確認する。editorではPPQ換算、基準tick、複数channelの統合、note以外の保全、tempo指定、SMF再読込、エラー時の原状維持と、キーボードでの入力・即時反映・試聴・移動・確定・Escでの原状復帰・Undo履歴の保全と、確定後のカーソル前進・Undo 1段での復帰を検証する。
