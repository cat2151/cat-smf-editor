# ADR 0013: 所有元layerを参照する専用parameterカーブを音声時刻で適用する

- 状態: 採用
- 関連: [0005](0005-tempo-curve-in-conductor-track.md)、[0006](0006-instrument-layers-and-patch-state-cache.md)、[player 0037](../../../cat-plugin-player/docs/adr/0037-direct-parameter-automation.md)

## 目的と判断の経緯

TB-303系のCutoffとResonanceをノートと別trackの2次ベジェで動かしたい。
当初検討したMIDI CCカーブでは、音源側のマッピングに依存し、公開parameterそのものを選ぶ目的を満たせない。
そのため初版を音源の直接parameter制御へ絞り、音源名やparameter名で番号を推測しない方式を採った。

既存のinstrument layerはtrackごとに独立した演奏instanceと共通effectを持つ。
automation trackにも音源を作ると、元のノートが鳴るinstanceへ値が届かず、gainとeffectも二重化する。
音源グループをconductorのregistryへ全面移動する案は既存のpatch復元とmixerの変更を広げるため採らず、
所有元trackの安定group ID、既存layer ID、plugin format/ID、parameter stable IDを専用trackから参照する。
同じ音色を使う別layerは別対象のまま保持する。

## 保存と編集の決定

format 1のtick 0専用metaにgroupとcurveを追加し、既存layer・tempo・masterのschemaは変えない。
実利用曲に新metaがないことを調べたうえで、旧曲を一括書換えする移行は不要と判断した。
曲のノート・未知meta・絶対tickと終端を保全し、必要なownerだけにgroupを割り当てる。
表示名とpathは補助情報であり、再起動後の照合に列挙indexや表示名を使わない。

tempoとparameterで共通なのは2次ベジェの幾何とpoint/control編集である。
tempo固有の実event生成規則や、最後のpointを消して生成tempoを残す挙動は共有しない。
parameterは空curveでもtargetを残し、適用を解除する。非空curveの競合や未解決targetには暗黙の勝者を設けず、
関係するownerの再生要求を理由付きで止める。無関係なscopeの再生や曲の確認・保存を妨げない。

対象選択は既存layer操作へ統合し、音源由来の名前とpathを共通Migemo検索で選ぶ。
初期点はカーソルtickと現在値とし、確定前に曲・layer・plugin・Native世代を再照合する。
値空間はCLAP plainとVST3 normalizedを区別する。値表記はUI側で音源へ問い合わせ、
当初はtext-to-value APIがないためnative数値入力を音源由来の文字列と分けたが、
実操作を経たユーザーの判断で値の直接入力欄を撤回した。音源の単位から逆変換を推測せず、
音源由来の値表示、ドラッグ、および値域を基準にしたキーボード増減を使う。

macro固定値編集の検討では、音源が宣言するbipolar値とAPIの公開値域・取得値が異なる例が見つかった。API値、公開値域内の操作位置、音源由来の表示文字列を混同しない共通境界を採り、表示値と操作位置を併記する。CLAP plainとVST3 normalizedの既存の保存形式・補間は変えず、表示文字列から単位を推測した変換もしない。範囲外の取得値は丸めず、新規作成と試聴を理由付きで拒否する。macro固有の宣言・復元の判断は[0014](0014-surge-macro-edit.md)へ残す。

## 音声時刻と基準音色の決定

tempoのNote On/Off tickだけの更新をparameterに使うと、持続音の途中で値が止まる。
描画frameやcallback開始点にも依存させず、曲の絶対sample位置に固定した64 frames格子で小数tickを評価し、
point・note・範囲境界でも分割する。同tickだけparameterをnoteより先に送り、
異なるtickが同sampleへ丸められた場合は元のtick順を保つ。native精度を超える時間差は作らない。

既存native APIは複数ID配送・値空間・RT処理に問題があり、専用の事前解決・容量固定経路を選んだ。
その根拠と補修の撤去条件はplayer ADRへ集約する。audio callbackではID検索・controller・文字列化・buffer成長を避ける。
容量不足を欠落として隠さず、対象を示して再生を止める。

seekとrepeatは開始tickの値を復元してからノートを送る。停止後の更新を止め、適用解除や通常演奏への再利用では
影響したlayerを保存済みpatch参照の基準stateから戻す。実行時のautomation値を音色cacheへ書き戻すと、
カーブを外した後も最後の値が基準音色になるため、保存先はcurveだけに限定する。
64 framesは初期値であり、fixtureの配送・CPU検査と実音源の聴感確認を区別して評価する。
初版は連続値の音源parameterに限定し、CC、VOICEVOX、effect/master、汎用MIDI合流は含めない。
