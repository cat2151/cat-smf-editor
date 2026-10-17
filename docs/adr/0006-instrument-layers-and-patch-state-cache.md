# ADR 0006: instrument layerは既存の演奏処理でUXを検証し、その後UAPMDの汎用グラフへ移行する

- 状態: 採用
- 関連: [0003: 共通hostとplayer選択](0003-shared-host-player-selection.md)、[0004: master effect](0004-master-effects-in-conductor-track.md)、[実装handoff](../../handoff-instrument-layers.md)

## 背景

現在のSMFでパイプオルガンの音色に不満があり、複数の音色を重ねて補いたい。
1つのtrackへ複数のpatchを割り当て、それぞれをoctave単位で移調し、volume dBで混合比を調整する。
このinstrument layerが主目的であり、patch参照によるSMF保存とstate cacheはそのための基盤である。
「起動後に最速で音を鳴らす」を優先し、復元のたびにplayerの起動やstate生成を待つ構成を避ける。
会話中の「status」は、この文書ではpluginの音色データであるstateを指す。アプリ設定の`status.json`とは別である。

## 調査した既存機能

UAPMDの手元checkout（調査時HEAD `a4a96938`）には、次の機能がある。

- `AudioPluginFullDAGraph`と`GraphConnectionExtension`: Event/Audioの接続を分岐・合流させる汎用DAG。
- `AudioPluginFullDAGraph.cpp`: 複数音源への入力event転送と、接続元の音声出力の加算。
- `GainNode`: 音量調整。単独での利用も可能。

該当箇所は兄弟checkoutの`source/uapmd-graph/`。調査したgraph・engine・data内では、
octaveずらしを含む専用instrument layer APIは見つからなかった。これはUAPMD全体に機能がないという断定ではない。

editorの`src/play/audition_output.rs`は、1つの音声streamでtrackごとの音源を処理し合算する。
共通hostの`shared/chain.rs`と`shim/uapmd_shim.cpp`の現在の契約は、音源1つと直列effectである。
初版から汎用DAGへ置き換えるより、この所有権・同期・ミックスを拡張してUXを確かめる方が変更範囲が小さい。

音源patchのstate生成は、playerの`src/patch_preparation.rs`から既存の共通API
`prepare_catalog_clap_patch_state`を呼んでいる。patch形式によって、ファイル変換または一時pluginの生成・読込・state取得が必要。
effectにも一時pluginを使う生成経路があり、Surge XT Effectsは既存の事前生成state表を使う例外を持つ。
これらの知識をeditorへコピーしない。

## 決定

### layerと初版の信号経路

- 1つのtrackに複数のlayerを持たせる。各layerはpatch識別情報、整数のoctaveずらし、volume dBを持つ。
- 同じtrackの演奏・試聴ノートを各layerへ送り、pitchを`12 × octave`だけずらす。
  移調後にMIDI pitchの0〜127を外れるノートは鳴らさず、端のpitchへ丸めない。
- 同じpatchを複数layerへ割り当ててよい。演奏用pluginインスタンスと保持ノートはlayerごとに独立させる。
- volume dBは音声の振幅へ適用する。velocityの変更で代用しない。
- 初版は既存の音声処理を拡張し、layer出力を合算した後、track共通のeffectを1回通す。
  layer別effectは初版の対象外。
- 信号順は`layerごとのinstrument → layer gain → track内合算 → track effect → track gain → 全track合算 → master effect → Master auto gain → 出力`。
  既存の演奏範囲、再生位置、単一の音声時刻、ノート停止、master処理を維持する。
- 専用overlayを用意し、1行1layerでpatch選択、octave、volume dB、追加・削除を扱う。
  操作説明や状態説明の常設文は追加せず、選択状態は色・枠などで示す。

### SMFの設定と実行時stateを分ける

- layer一覧は対象trackのtick 0に専用のSequencer Specific Meta Event（`FF 7F`）として保存する。
  名前空間付きpayloadとversionを持つ既存方式を踏襲する。prefixは`\x7dcat-plugin-player\0instrument-layers\0`、versionは1。layerには安定ID、patch参照、octave、volume_dbを持たせる。
- 保存するのはpatchの復元に必要な識別情報とlayer設定であり、plugin state本体はSMFに保存しない。
  表示名だけで同名patchを選び直さない。plugin format/ID、patchの元データ位置・program等を失わない参照を使う。
- Dexedの参照にはprogram付き論理pathと元cartridgeの実ファイルpathを別々に持たせる。playerが既存coreのcartridge parserで実ファイルを特定し、editorはplugin別のpath解釈を持たない。programをcache keyに残し、実ファイルの更新でstateを無効化する。
  SMFに書くpathの形式（root相対・`/`区切り）は[0008](0008-machine-independent-patch-paths-in-smf.md)に従う。
- effect・master effectにも既存browserで選んだpresetを復元できる参照を渡し、state非保存と整合させる。
  effectの配置は既存track/masterの区別を維持する。
- ノート・tempo・未知イベント・他trackを変えず、絶対tickを維持して必要なdeltaだけ再計算する。
  原本保護と既存savedコピーの衝突・外部変更保護を維持する。
- 互換性は要求しない。旧state保存形式の移行frameworkや旧形式へのfallbackは作らない。
  不正metaや復元不能なpatchを、別の音色へ黙って置き換えない。

### playerとcacheによる復元

- state生成はplayerの既存処理へ集約する。editorへplugin別変換・読込・例外処理を複製しない。
- browserからeditorへpatch参照も渡す。選択時に生成済みのstateを渡す既存経路は活用してよい。
- editorは必要なpatchのstateをメモリで保持し、アプリ終了時にそのsessionで使用したstateを`%LOCALAPPDATA%/cat-smf-editor/cache/patch-states-v1/`へcacheする。
  現在開いている曲・最後のpatchだけに限定しない。
- SMF復元ではcacheを先に引き、揃っていればplayerを起動せず、カタログ走査やstate再生成も行わない。
  cacheにない、破損している、または無効になったpatchだけplayerへstate生成を依頼する。
- cacheはpatch単位。同じpatchの複数layer・trackでstateデータを共有できるが、演奏用インスタンスは共有しない。
  octave・volume dBはstate生成の入力ではなく、cache keyへ混ぜない。
- cacheは元patchとplugin bundleのファイル長・更新日時（bundleがdirectoryならその配下の情報も）を記録し、state本体のchecksumも検証する。
- patch・pluginの更新時に誤ったcacheを採用しない。無効化の判定方法は起動速度と整合させ、毎回の全面走査を前提にしない。
  cacheは再生成可能なデータとして扱い、書込み失敗や生成失敗を隠さない。
- この永続state cacheと、[既存の非使用中pluginインスタンスcache](../plugin-instance-cache.md)を区別する。
  state cacheは次回起動用のバイト列であり、pluginインスタンスを終了後まで保持するものではない。

## UX検証後は汎用グラフへ進む

初版の既存ミックス拡張は、instrument layerのUXを短い経路で検証するための段階的な実装である。
これを最終構成として固定せず、UX検証が進んだ後はUAPMDの汎用グラフへ移行する。

- overlayでの音色追加・交換・削除、octave・dB調整、パイプオルガンでの聴取、保存復元、起動後の発音を人間が確認する。
- その結果を踏まえて汎用グラフへの移行を別の実装範囲として具体化する。初版handoffの自動継続で実装しない。
- 移行先ではMIDI分岐、layerごとの移調とgain、音声合流、track共通effectを表現する。
  音符スケジュール・時刻管理とstate生成を二重実装しない。
- 移行時にはNote On/Offの対応、範囲・停止、effectの適用回数、音量、plugin latency、所有権・破棄順を確認する。
  保存モデルとoverlayの操作意図をgraph内部表現から分離し、UX検証結果を引き継ぐ。
- 汎用graphを採用することと、汎用のnode編集UIを公開することは別。後者やlayer別effectは今回承認された機能に含めない。

## 検証と現在の状態

機械検証は、layer間の独立性、octaveとdB、共通effectの順序、SMFの参照のみの保存、
cache命中時のplayer未使用、cache不在時のplayer復元、既存保存保護を対象とする。
初版実装の完了と、人間によるUX・聴取確認の合格は別に記録する。

参照保存、永続cache、非表示生成要求、layer演奏経路、専用overlayを実装した。
参照encode/parse・cacheの新規session復元・対象照合・入力抑止に加え、実Surge XTの複数layer発音と独立した停止、実browser選択、overlayの追加・交換・削除・数値編集、cacheからの実アプリ再起動復元を検証した。
実session復元から既定音声出力の最初のcallbackまで、Organ 1の2layerでcacheなし約1.26秒、cacheあり約1.21秒だった（復元部分は約99ms／1.4ms）。warm側は存在しないplayerパスを明示して成功し、生成待ちを除けることを確認した。この比較はGUI準備前のsession復元開始を起点とし、OSのプロセス起動時間や人間の聴感判定を含まない。
人間のUX・聴取確認は未完了であり、handoff全体の合格を意味しない。実行条件、最新の検証証跡とStage状態は実行ledgerで管理する。
