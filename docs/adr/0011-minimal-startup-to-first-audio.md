# ADR 0011: 最初の発音までのstatus・cache読み込みと処理を最小化する

- 状態: 採用。最小statusと演奏前後の経路分離を実装。Surgeの音源生成は[ADR 0012](0012-surge-minimal-data-home.md)で追加最適化。cache検証方式の変更は未実施。
- 関連: [0006: layerとstate cache](0006-instrument-layers-and-patch-state-cache.md)、[0008: patch pathとroots cache](0008-machine-independent-patch-paths-in-smf.md)、[0009: 初回依存物導入](0009-first-run-dependencies.md)

## 背景

アプリ起動から音が鳴るまでの時間の最小化を、起動設計の優先事項とする。
ADR 0006にはcache命中時のplayer起動・state再生成を避ける方針があるが、アプリのstatus読み込み、画面準備、初回導入判定まで含めた処理順は定めていなかった。

ここでstatusはアプリの復元情報、stateはpluginへ読み込む音色データを指す。
最小化の対象は、元の曲・音色・layer・effect・音量・tempoを守った演奏を開始するまでの経路である。別の簡易音色を先に鳴らして達成扱いにしない。

## 決定

### 発音までの最小経路

通常のcache命中起動を、次の依存関係で構成する。

1. CLIを解析し、明示されたSMFを優先する。指定がなければ、最後のSMFを特定するための最小statusだけを読む。現行の起動時演奏は全trackを先頭から鳴らすため、カーソル・選択track・分割画面・browser・解析窓の復元情報はこの段階で読まない。
2. 対象SMFを読み、演奏に必要なイベント、tempo、noteの対応、音色参照、layer、track/master effect、gainを解決する。native libraryなど、この演奏に不可欠な基盤を用意する。
3. この演奏に必要なpluginを特定するroots情報と、そのpatch/effectのstate cacheだけを読む。全cacheの列挙・読込や別曲の準備をしない。演奏に寄与しないtrackの音色復元は後回しにする。
4. 必要な音源・effectとaudio deviceを用意し、既存の演奏時刻・ノートスケジュールの仕組みで演奏を開始する。別の起動専用演奏器は作らない。
5. audio callbackが演奏データを処理し始めたことを確認してから、発音に不要な復元・保存・準備を進める。

最小statusは、用途だけでなく読込単位も分ける。大きな`status.json`を全readして小さな型へdeserializeするだけでは、不要なreadを除けない。独立した小さな永続レコード等で、発音に必要な情報だけを取得できるようにする。具体的な形式・ファイル名は実装時に決める。SMFをCLIで指定した場合は、最後のSMFの特定用statusも読まずに進める。

小さな最小statusはユーザーの復元情報であり、消してよいcacheとして扱わない。SMF・音色参照を正本、stateとrootsを再生成可能なcacheとする既存の区別を守る。

### 発音後の処理

以下は最小経路の前提条件にしない。

- 残りのstatus、履歴、browserのpane・query・patch cursorの読み込みと復元。
- 選択track、編集カーソル、分割画面、window配置、解析窓の復元。
- 表示用のtrack名、調号、表示行、時間列などの構築。演奏に必要なnote対応やtempo計算と分離し、重複計算も避ける。
- 保存時の比較用SMF再encode、履歴の長さ計算・記録、statusの保存と`sync_all`。
- 日本語fontの読み込みなど、最初の発音に不要なGUI準備。UI frameworkの制約上残る待ちは計測で明示する。
- 演奏に不要なplayer探索・導入確認、plugin一覧cacheの読み込み、音源の全面検出、別曲のpreview作成。

単にworkerへ渡して発音前から走らせるのではなく、開始自体を後回しにする。発音後もUI threadやdisk/CPUを占有して音切れを起こさないよう小分けに進め、pluginの生成・pump・破棄に関する既存のthread制約を守る。
後回しのstatusを読む前にdefault値で保存して既存値を消したり、復元時に開始済みの演奏を再起動したりしない。ユーザー操作が先行した場合は、その操作を遅れて届いた復元値で上書きしない。

### cache不備・依存物不在・演奏不能の場合

- cacheが有効で演奏可能なら、playerやplugin一覧cacheの不在を発音の障害にしない。
- rootsやstateが欠ける・破損・無効の場合だけ、必要な参照の解決やstate生成にplayerを使う。必要な実音源が不在なら初回導入へ進む。全体の検出や導入は、その演奏に必要なときに行う。
- ADR 0009の「起動時に依存物確認を先行する」順序は、この最小経路に必要な場合へ限定する。導入元、同意、既存exeの保全など、導入自体の規則は維持する。明示した`--player`が不正な場合のエラーも維持し、不要な事前起動・導入待ちにはしない。
- cache検証は必要な参照に限定し、同一pluginの検証を共有する。現行のbundle再帰走査を毎patch反復する方式は最適化対象とする。高速化のために更新されたpatch/pluginの古いstateを黙って使わない。無効化の正しさを保つ検証方式は、実装時に選定・検証する。
- 対象SMFなし、読み込み失敗、無音の曲、audio deviceの失敗時には、発音確認を永久に待たず、必要なUI・ファイル選択・エラー処理へ進む。ユーザーの停止・終了要求も妨げない。
- 曲の冒頭に休符がある場合は、その音楽上の待ちを維持する。最初のcallbackと最初の非無音出力を区別し、休符が終わるまでUI復元を止め続けない。

## 2026-10-19の変更前の実装確認

作業ツリーのコードを静的に確認した。以下は経路と依存関係の確認であり、各処理の実測時間や遅延寄与の順位ではない。

| 箇所 | 現在の発音前の処理 | 判定 |
| --- | --- | --- |
| `src/main.rs::run`、`src/app.rs::App::new` | native library、window設定、GUI作成、日本語font、status全体、履歴を準備する | 演奏とGUIの準備が未分離 |
| `src/player/setup.rs::Setup::begin` | player解決とplugin一覧cache確認を先行し、条件によってscan/導入完了までSMF復元を待つ | cacheだけで演奏できる起動にも不要な依存を作る |
| `src/store/session.rs::restore_session` | 曲を開いた後、編集位置・分割画面等を復元する | 全track先頭演奏には不要 |
| `src/store/saved.rs::Document::load` | SMFのread/parseに続けて保存比較用に再encodeする | 保存準備を演奏前に実行 |
| `src/app/sound.rs::resolve` | masterと全trackの音色・effect・音量を復元する | 発音しないtrackの参照も解決 |
| `src/score/state_cache.rs::Cache::resolve` | patch単位で必要になったcacheをreadし、命中時は生成を呼ばない | 必要分readの基盤はある |
| 同ファイルの`fingerprint` / `stamp` | 参照ごとに元patchとpluginを検証し、directory形式bundleでは配下を再帰走査する | cache命中でも走査があり、同一bundleの検証も反復 |
| `src/app/loading.rs::install_resolved` | 全trackの表示データを構築し、`NoteView`も表示行用と保持用に二度構築する。履歴の長さ計算・保存、status保存も行う | 表示・永続化が再生準備に先行 |
| `src/app.rs::App::new`、`src/player/setup.rs::show` | 解析窓のworker復元が`playback::start`より先 | 発音後へ移せる |
| `src/play/playback.rs::start` → `src/play/transport.rs::advance` | startは演奏準備をpendingにし、後続updateで音源とdeviceを開く | start呼出時点を発音開始扱いにできない |

通常起動は`open_initial` → `open_mid`の同期経路であり、`src/view/opening.rs`の描画を挟む段階読み込みとは別である。後者も表示データ構築と保存をaudio準備より先に行うため、そのまま起動へ転用しても分離は達成しない。
一方、別曲previewのprefetchはrecent画面が開いている場合に限定されており、通常起動で常に走る処理ではない。

この時点では、patch単位cacheの優先利用というコンセプトは残っていたが、起動全体を「最小status → 必要cache → 演奏 → その他」にする分離は維持できていなかった。

## 実装と実測

### 最初に実装した範囲

- `src/app/startup.rs`で、native library準備後、GUI生成前に対象SMFと必要な音色を復元して演奏する。通常の演奏器とaudio callbackを使い、最初の演奏callbackを待ってからGUI準備を始める。callback待ちは最大10秒とし、失敗時はGUIで既存のエラー処理へ戻る。
- 最後のSMFのpathだけを`%LOCALAPPDATA%/cat-smf-editor/startup.json`へ保存し、通常起動の発音前にはこれだけを読む。CLIでSMFを指定した場合は、このreadも省く。`status.json`・履歴・window設定・font・解析窓・編集位置の復元と、履歴/statusの保存を後段へ移す。
- `status.json`の`navigation.last_mid`は、そのUI/session snapshotの対象曲を照合するために残す。起動する曲の正本は`startup.json`とする。2ファイルの保存途中で終了した場合も、別の曲のカーソルを適用しない。同じfilenameでもdirectoryが違えば別の曲として扱う。
- 発音しないtrackの音色復元、表示行・時間列・track名の構築、保存比較用SMF再encodeを後回しにする。noteの対応・演奏準備に必要なindexは先に用意し、表示行用と保持用の`NoteView`の二重構築を解消する。
- 演奏可能な起動ではplayer探索やplugin一覧cacheの確認をしない。復元できない起動だけ既存のsetupへ進む。壊れた後段statusをdefault値で上書きせず、復元完了前に編集操作へ入力を渡さない。

### 計測結果と優先順位

Windowsの既定音声device、release build、実際に前回開いていた`concerto-in-e-major-mids/score.mid`（6track、うち5trackが発音、約59KB、既定音源）を使用した。各回は実設定・cacheを隔離先へ複製した新規プロセスであり、SMFとpluginは実パスを使う。OS・disk cacheを排出したcold bootの測定ではない。

| プロセス起動から最初の非無音出力blockまで | 1回目 | 2回目 | 3回目 | 中央値 |
| --- | ---: | ---: | ---: | ---: |
| 変更前 | 3685ms | 3641ms | 3643ms | 3643ms |
| 経路分離後 | 3362ms | 3343ms | 3286ms | 3343ms |

中央値で約300ms、約8%短縮した。主要因はGUI生成を演奏後へ移したことであり、約8KBのstatus分離だけの効果ではない。変更前の表示構築と履歴/status保存は合計約23msだった。変更後のtraceでは、最小statusのreadが約1msで、全status・履歴・表示構築・保存・fontは演奏callbackより後に始まる。

残る支配的な処理は5つの音源chain生成で、合計約3.2〜3.3秒である。音声deviceを開かない追加計測でも、空のhost生成は約0.7〜2.2ms、既定音源chainは約677〜685msだった。trackごとの独立性、音色適用、初期noteを維持する必要があるため、単純な待ちの削除や音源の共有は行わない。さらに大きく短縮する場合は、この生成経路の内訳と共通host側での再利用可能な資源を先に調べる。

音色設定済みの実曲`modified/ToccataFugue.mid`もCLIで確認し、9件のstate解決をcacheから約35msで完了し、プロセス起動から非無音出力まで約1271msだった。`state_fingerprint`は各約0.1msであり、この実データではcache検証の方式変更より経路分離を優先した。directory形式bundleの全面走査削減は未実施で、速さのために無効化検証を外す変更もしていない。

ここでの「非無音出力」は、sequence commandを処理したaudio callbackのmixに有限な振幅が現れた時刻であり、スピーカーからの音を外部収録した時刻ではない。最初のcallbackと非無音出力は別々に記録する。物理deviceのbuffer遅延と、人間による聴感の合否はこの数値に含めない。

### Surge生成の追加最適化

この後、play-serverの`SURGE_DATA_HOME`設定を参考に、発音に不要なpatch directory走査を省いた。同じ曲・新しいexeで最適化の無効／有効を各3回測り、プロセス起動から非無音出力までの中央値が3363msから1132msになった。経路分離前の3643msからは約69%短縮である。資源の保持、子playerへ渡す環境、音声比較、初回cache生成を含む詳細は[ADR 0012](0012-surge-minimal-data-home.md)に記録する。

FX preset・wavetablesも最小化したv2構成は、出音比較後に追加採用した。同じ新しいexeでv1／v2を各3回測定し、中央値は1106msから280msへ短縮した。111ケースのstate復元比較と、波形が安定した42ケースの全sample一致を確認している。詳細は[追加検証の記録](../measurements/surge-resources-audio-20261019.md)を参照する。

### 実利用での聴感確認とUX評価（2026-10-19）

ユーザーが普段使う曲での聴感確認を完了し、最適化の効果が実利用で発揮され、UXが劇的に改善されたと報告した。起動から発音までを最小化する方針について、機械計測による時間短縮に加え、ユーザー自身による聴感確認と実利用上の効果を確認できた。

### 再測定

`scripts/measure_startup.ps1`で同じ前回曲と設定/cacheを毎回隔離し、終了は通常の保存・終了経路を使う。起動前の情報を元データへ書き戻さない。traceは環境変数があるときだけ有効で、callbackではatomicな時刻記録だけを行う。ファイル出力はUI threadで行う。

```powershell
cargo build --release
./scripts/measure_startup.ps1 -Runs 3
./scripts/measure_startup.ps1 -Runs 1 -Midi 'X:/music/song.mid'
```

出力先の`results.json`と各回の`trace.json`に、プロセス生成からmainまで、mainから演奏callback・非無音出力まで、各処理の開始時刻と所要時間を残す。`CAT_SMF_STARTUP_TRACE`を出力先pathに指定すれば実利用データの起動も測れる。計測用の`CAT_SMF_STARTUP_TRACE_QUIT=1`は後段の復元と記録が完了した後、通常終了を要求する。未指定の通常利用では自動終了しない。

## 検証条件

- プロセス起動から、最小status/SMF/cacheのread、plugin準備、演奏データ処理開始callback、最初の非無音出力までを分けて計測する。曲冒頭の休符と音声device固有の待ちを別記する。
- cache命中の通常起動で、残りのstatus・履歴・plugin一覧cache・無関係なpatchを発音前に読まず、player生成要求、全面scan、履歴/status保存も行わないことをI/Oと処理順の記録で検証する。単なる総時間の短縮だけを合格条件にしない。
- CLI指定、最後の曲の復元、player不在でのcache命中、cacheの欠損・破損・更新、複数layerとtrack/master effect、無音の曲、ファイルなし、出力失敗、停止・終了を確認する。
- 発音後の遅延復元が演奏を途切れさせず、設定・音色・編集内容・ユーザーの先行操作を失わないことを確認する。
- 既存の`startup_session_cache_restores_layers_and_reaches_audio_without_player`は`App::new` / `Setup::begin`を通らず、計測もstatus読み込み後の`restore_session`から始まる。ADR 0006の約1.21秒はこの部分経路の値であり、通常のプロセス起動全体の達成証拠には使わない。

## 保存形式と実データの移行

実利用中のstatus、履歴、settings、復元対象SMFを確認し、`%LOCALAPPDATA%/cat-smf-editor/backup/startup-status-20261019-185235/`へ元のファイルとSMFのpath/checksumを退避した。既存statusの`navigation.last_mid`から112バイトの`startup.json`を一度限りで作成し、status本体は変更せず残した。SMF・state cache・音色参照の形式は変更していない。

移行した実データで、引数なしの通常GUI起動から非無音出力、後段の復元、通常終了、再起動を2回確認した。元のstatusの全項目、settings、履歴の対象と内容（通常起動で更新するopened時刻を除く）、SMFのchecksumを照合して保全を確認した。隔離テストでは、後段statusの破損、CLI優先、無音のintro、停止、別曲のUI snapshot、最初のフレームのショートカットも確認する。

旧statusからの自動fallbackや恒久的な互換層は追加しない。`startup.json`がない新規環境では前回曲なしとしてファイル選択へ進む。移行を戻す場合は、新規追加した`startup.json`を退避し、必要ならバックアップのstatusを戻すことで元のデータへ復元できる。
