# envでFX preset・wavetablesを最小化した結果（2026-10-19）

## 結論

`SURGE_DATA_HOME`でFX preset・factory／3rd-party wavetablesを空directoryにした構成を指定すると、今回確認したstate・元FXP・内蔵FXの復元を保ちながら高速化できた。Templatesは元directoryのまま保持する。editorの演奏用cacheにはこの構成をv2として採用した。

通常起動の実曲・既定Surge 5音源では、起動から最初の非無音出力blockまでの中央値が1106msから280msへ約75%短縮した。音声deviceの物理遅延や外部収録による発音時刻は含めない。

関連: [ADR 0012](../adr/0012-surge-minimal-data-home.md)、[直前の生成内訳調査](surge-creation-20261019.md)、[各回の数値と音声比較集計](surge-resources-audio-20261019.json)。知見を提供した[play-server ADR 0010](../../../clap-mml-play-server/docs/adr/0010-surge-data-home-and-plugin-identity.md)の起動時間最小化方針へも、成功結果と限界をフィードバックした。

## 試した構成

`probe_surge_resources.py`で用意した独立した測定用directoryを子プロセスのenvへ渡した。元のインストール先・実cache・ユーザー／システム環境変数は変更しない。

| 構成 | FX preset | wavetables | Templates | 5音源のcreate合計・直前調査の中央値 |
| --- | --- | --- | --- | ---: |
| 比較元v1 | 元directory | 元directory | 元directory | 909ms |
| FXだけ最小化 | 空directory | 元directory | 元directory | 498ms |
| wavetablesだけ最小化 | 元directory | 両directoryとも空 | 元directory | 530ms |
| 両方を最小化 | 空directory | 両directoryとも空 | 元directory | 129ms |

音声比較はWindows x64、前調査と同じSurge XTバイナリ・UAPMD host、48000Hz、各block 1024 frames、各case 128 blocksで行った。noteは48・60・67、velocity 90、duration 96000 frames。単体FXには220Hz・振幅0.25のsineを入力した。音声deviceは開かない。

## 音色・FX・元FXPの比較

実利用の`patch-states-v1`から48件のSurge音色と14件のSurge XT Effects stateを隔離fixtureへ複製した。48音色については、元FXPの60-byte containerとsub3 payload長を検証して取り出したstateを直接ロードするcaseも作った。既定音色を含めて111 caseである。実cacheや元FXPは書き換えない。

各caseを元構成で2回、さらに独立した元構成プロセスでも1回renderし、各候補構成を2回ずつrenderした。

| 候補 | 有限・非無音 | 復元後state一致 | 元構成でも波形が安定したcaseの全sample一致 |
| --- | ---: | ---: | ---: |
| FXだけ最小化 | 111 / 111 | 111 / 111 | 42 / 42、候補の2回とも |
| wavetablesだけ最小化 | 111 / 111 | 111 / 111 | 42 / 42、候補の2回とも |
| 両方を最小化 | 111 / 111 | 111 / 111 | 42 / 42、候補の2回とも |

state比較では、`lastLoadedPath`の値だけを実際の参照先へcanonicalizeした。最小構成のjunctionを通ったpathと元pathの文字列差を、音色差として数えないためである。それ以外のXML、wavetable長、wavetable binary payloadは省略せず比較した。Surge XT Effectsのstateはそのままbyte比較した。

当初は元構成の同一プロセス内2回だけで波形が安定した46 caseを選んだが、別プロセス比較ではOrgan 2・Churchのcache／元FXPの4 caseが変動した。元構成の独立再実行でも変動することを確認し、候補だけの回帰とは扱わない。最終集計の「安定42 case」はこの再確認後の数である。残り69 caseについてbit一致や聴感一致まで保証したとはしない。

Init Wavetable・Init FM2・Init Modernと既定音色を別fixtureでも各構成2回ずつ確認した。全4 caseの復元後stateが一致し、有限・非無音だった。このうち元構成内でも波形が安定した1 caseは各構成でも全sample一致した。Init Wavetableの元FXPには6 oscillator分のwavetable payloadが含まれていた。

UAPMDのstate取得経路は、全構成に共通して`params.flush() on wrong thread`の診断を出した。hostの補修は今回の範囲に追加せず、取得成功・復元値・音声を同じhost条件で比較した。これはhost全体のthread規約の合格を示す検証ではない。play-serverの別hostへ結果を適用するときにも、その経路で再確認する。

## 通常アプリでの実測

同じ新しいrelease exeでv1構成をenvに明示した条件と、v2を通常利用する条件を比較した。`measure_startup.ps1`が実設定・cacheを隔離先へ複製し、元SMF・pluginを実パスで読む。OS・disk cacheを排出したcold boot測定ではない。

| プロセス起動→最初の非無音block | 1回目 | 2回目 | 3回目 | 中央値 |
| --- | ---: | ---: | ---: | ---: |
| patchだけ最小化したv1 | 1106ms | 1114ms | 1057ms | 1106ms |
| FX・wavetablesも最小化したv2 | 280ms | 280ms | 287ms | 280ms |

さらに実利用データを直接使って通常起動・終了・再起動を2回確認した。main開始から非無音出力までは、v2 cacheの初回生成込みで405ms、cacheありの再起動で216msだった。この2値は上表のプロセス起動からの値とは区別する。

実行前のstartup/status/settings/historyを一時保存して照合し、startup/status/settingsはbyte一致、SMFはchecksum一致、履歴はopened時刻以外の全内容が一致した。既存v1 cacheを残し、新しい`cache/surge-data-min-v2/<key>/`を作るだけで、設定・音色参照・SMFの形式移行は行っていない。

## 実利用での聴感確認とUX評価（2026-10-19）

ユーザーが普段使う曲の聴感確認を完了し、最適化の効果が発揮され、UXが劇的に改善されたと報告した。この確認は採用後のeditorに対するユーザー自身の評価であり、上記の機械計測・state比較・波形比較に追加する実利用上の成果である。

## 採用範囲と限界

- 演奏用instanceだけに最小構成を適用する。browser・roots探索・state生成を行う子playerには元のenvを渡す。元patch・FX preset・wavetableはインストール先に残す。
- Templates・modulator preset・skins・tuning等の既存共有資源は保持する。新しい最小構成の生成・検証に失敗したら、元のenvのまま通常構成で続行する。`CMRT_SURGE_MINIMAL_DATA_HOME=0`も維持する。
- 明示された既存v1／play-serverの最小data homeは従来構成として検証し、その指定を維持する。通常起動のv2 cacheとは区別する。
- 今回確認したFXP／stateではwavetableがpayloadへ埋め込まれていた。普段使う曲の聴感確認はユーザーにより完了済みである。未知の外部wavetable参照、すべてのpatch、将来のplugin版まで検証範囲を広げたものではない。
- play-serverはADRへのフィードバックだけで、実装は変更していない。このeditorのMIDI経路の比較を、そのままserverのCLAP note dialect経路の検証済み扱いにはしない。

## 再実行

まず`cargo test --release --lib surge_data_home_audio_probe --no-run`で診断テストをビルドする。fixture作成と各候補treeの作成は、元データの外側にある新規directoryを使う。

```powershell
python scripts/prepare_surge_audio_fixtures.py --cache 'X:/app/cache/patch-states-v1' --output 'X:/measurements/audio-fixtures' --templates 'X:/Surge XT/patches_factory/Templates'
python scripts/probe_surge_resources.py --data-home 'X:/app/cache/surge-data-min-v1/key' --output 'X:/measurements/resource-trees'
```

診断テストは、子プロセスの`SURGE_DATA_HOME`に候補tree、`SURGE_AUDIO_PROBE_INPUT`にfixtureのinput、`SURGE_AUDIO_PROBE_OUTPUT`に構成別の新規出力先、`SURGE_AUDIO_PROBE_REPEATS=2`を設定して実行する。テスト名は`startup_trace::native_audio::surge_data_home_audio_probe`、引数は`--ignored --exact --nocapture`。元構成は独立プロセスの再実行も1回行う。テスト実行ファイルのhash付きfilenameはビルド結果から取得する。

```powershell
python scripts/compare_surge_audio.py 'X:/measurements/baseline' 'X:/measurements/candidate' --baseline-recheck 'X:/measurements/baseline-recheck' --output 'X:/measurements/comparison.json'
```

比較scriptは結果をJSONへ出し、元構成の独立再実行でも安定したcaseと変動するcaseを分ける。通常アプリの時間は`measure_startup.ps1`で別に測定する。
