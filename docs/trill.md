# trillの管理と再配置

## 操作と保存

tempo curve画面を開くと、Reference Tempoの初期化と同じタイミングで全trackのtrillを検知する。
検知したmetaは表示用に仮生成し、閲覧だけでは曲・保存待ち内容・Undo履歴を変えず、ノートも再配置しない。
カーブ・目安テンポの実編集、またはtrillの変更確定時に、必要な初期化をその編集とまとめて記録する。
既にTrill Start / Endの組があれば再検知せず、保存済みの値を使う。
曲中の`Trill Start`／`Trill End`のtext（tempo curveの上下両方の描画領域）、
またはevent listのtrill meta（Notes／Raw）をクリックすると、対象trillの編集画面を開く。
ピアノロールのtrill textと、textの代わりのアイコンも同じ編集に対応する。

`First pitch`の`Upper`／`Lower`で開始音を選ぶ。
`First note`で最初の1音の長さ、`Interval`で2音目以降の発音間隔を、秒で個別に入力・ドラッグ変更する。
開始音の選択、入力・ドラッグのたびに開始metaとノート配置を保存コピーへ反映する。原本は上書きしない。
再生とA–B repeatは、現在の演奏tick・終了位置・対象track・リピート範囲を保ち、新しい配置で続行する。
`Apply`で確定して閉じ、初期化を含めUndo 1回分にする。値を変えずに確定した場合は何も記録しない。
`Cancel`／`Esc`／外側クリックでは初期化・変更・Undo履歴を残さず編集前の曲へ戻し、演奏を続ける。
不正な値では最後の有効な配置と再生状態を保ち、編集画面にエラーを表示する。

検知条件は、同じtrack・channelで半音または全音の2音が6音以上交互に現れ、
64分音符以下の等tick間隔で、各Note Offが次の発音位置と一致すること。
別音高の伴奏は同じtrack・channelにあってよい。同じ音高を共有して重なる候補は除外する。
これはSMFに展開済みの音型を拾う条件であり、演奏の揺れや全ての楽譜上のtrillに対応するものではない。

SMF format 1 / PPQNのみを扱う。Text meta (`FF 01`) をノートのあるtrackへ保存する。
例（開始metaのtickが開始位置、終了metaのtickが終了位置）:

```text
Trill Start: {"version":1,"id":1,"channel":1,"notes":[65,67],"interval_seconds":0.06621825,"first_note_seconds":0.2,"pattern":{"first_note":67,"velocities":[90,90],"release_velocities":[0,0]}}
Trill End: {"version":1,"id":1}
```

- idはtrack内で一意、channelは1始まり、notesは昇順のMIDI note number。
- interval_secondsは2音目以降の各音から次音までの発音間隔。2音で一往復する時間ではない。
- first_note_secondsは最初の1音から2音目までの秒数。省略時はinterval_secondsを使う。
  編集画面で値を変更すると両方の秒数を明示保存する。最初の値を変更しても以後の間隔は変わらない。
- 初回は最初のNote Onから次のNote Onまでを、その時点のtempo eventを積算して秒にする。
  途中にtempo変更があればその両側を積算する。カーブが既にある曲では、そのカーブから生成済みのtempo eventを使う。
- patternは最初の再配置時に採取する。開始音と音高ごとの最初のvelocity・release velocityを保持する。
  開始音の選択はpattern.first_noteへ保存する。音高ごとのvelocityは保ち、後の再配置でも繰り返す。Note Offは`8n`形式で生成する。
- 画面の再表示やカーブの変更では秒間隔を取り直さない。編集画面での反映だけがその値を変更する。
- 先行実装でpatternがないmetaは、最初の再配置時に既存の2音から採取する。
  まだpatternがなく、片方の音が既に失われている場合はエラーにする。

## 再配置

開始・終了tickは固定する。最初の1音をfirst_note_seconds（省略時はinterval_seconds）だけ持続し、
2音目以降はinterval_secondsの間隔で交互に発音する。各音は次の発音まで持続させる。
最後の音は終了tickで切る。終了位置ちょうどの次音は生成しない。
区間の実時間が変わるとノート数が増減する。最初の音の長さが区間より長ければ最初の1音だけになる。
1音になってもpatternが残るため、間隔を短くすれば元の2音と開始音を復元できる。

対象は開始以上・終了未満の、指定channel・2音高に属するNote Onと、それに対応するNote Off。
別音高・別channel・範囲外のノート、CC、他metaのtickとデータは保持する。
同tickの消音は発音より前に置き、controller等は発音より前、End of Trackは最後に置く。
これにより、区間直後の同音高ノートや隣接するtrillが消音されるのを防ぐ。

生成ノートの単独移動・長さ変更はNotes表示とピアノロールの両方で拒否する。
生成ノートを範囲外へ移してしまうと、移動した音と元の範囲で再生成した音が二重に残るため。
伴奏は従来どおり編集できるが、trillと同音を重ねる変更は拒否する。

## テンポ生成との整合

[ADR 0005](adr/0005-tempo-curve-in-conductor-track.md)のとおり、tempo eventは
全trackのNote On / Offの位置とtick 0に、カーブのBPMを整数µs/quarterへ変換して生成する。
ノートを動かすとtempoの採取位置も変わるため、古いtempo mapで配置してからtempoを更新するだけでは秒間隔がずれる。

再配置は次の方法で行う。

1. 元の管理対象ノートを外し、伴奏の全Note On / Offと各trillの開始・終了位置を固定境界とする。
2. 時間順に境界を進め、その位置でのカーブ値を、出力と同じ整数µs/quarterへ変換する。
3. 次の発音目標を「開始の実時間 + 最初の1音の長さ + (発音番号 − 1) × 秒間隔」
   （発音番号は0始まり、最初の音だけ開始位置そのもの）で求め、現在のtempoでtickへ換算する。
   先に伴奏や別trillの境界へ着けば、そこで更新されたtempoで残り時間を計算し直す。
4. 再配置した全ノートの最終位置からtempo eventを生成する。

発音間隔ごとの丸め誤差を次へ足さず、常に開始からの目標時刻で合わせる。
SMFは整数tickなので、秒間隔にはtick精度の誤差が残る。
カーブがなければ既存tempo eventを時間源に使い、tempo eventは書き換えない。
カーブがある場合、conductor以外のtempo eventとの併用はエラーにする。

再生成は通常編集の確定後とtrillの値の変更時に行う。tempo curveを開くだけでは行わない。保存・再読込後も同じ配置になる。
ドラッグ中のカーブプレビューは現在ノート位置のtempo表示であり、trillの再配置は確定時に反映する。

## 拒否するデータ

非正・非有限の秒間隔、同じtickに複数の発音が必要になる速度、不正な開始終了meta、
同じtrack・channel・音高の領域重複、境界をまたぐ同音、未対応のNote Off、
管理対象内で重なるノートはエラーにする。1回に生成するノートが合計100万音を超える場合も停止する。
再生成全体をコピー上で行い、失敗時は元の曲・保存待ち内容・世代番号を変更しない。

## 検証

- `edit::trill::rebuild::tests`: tempo変更をまたぐ秒間隔、カーブと生成tempoの整合、複数trill、
  1音からの復元、終了位置の同音、再生成・保存再読込の一致、拒否時の無変更。
- `view::tempo_curve::tests::trills`: 初期化タイミング、上下両方のtextからの編集、ドラッグ編集とmeta保存、
  カーブ変更による再配置、不正な間隔を入力した編集の取り消し。
- `view::trill_edit::tests`: Notes／Rawの開始・終了meta、独立した最初の音の長さ、Apply／Cancel、
  マウスを押したままの変更反映、次のA–B周回への反映、Cancelで元の配置と演奏を復元。
