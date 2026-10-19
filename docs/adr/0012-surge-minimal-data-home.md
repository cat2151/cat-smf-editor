# ADR 0012: Surge XTの発音に不要なpatch directory走査を省く

- 状態: 採用・実装済み
- 関連: [0011: 最初の発音までの最小経路](0011-minimal-startup-to-first-audio.md)、[0006: state cache](0006-instrument-layers-and-patch-state-cache.md)
- 参考: [play-serverの実装](../../../clap-mml-play-server/core-lib/src/surge_data.rs)、[play-server ADR 0010](../../../clap-mml-play-server/docs/adr/0010-surge-data-home-and-plugin-identity.md)

## 背景

起動経路を分離しても、既定のSurge XTを5track分作るため約3.2秒かかっていた。音声deviceを開かないFFI単位の計測では、host生成やplugin情報復元ではなく、instance生成要求そのものが1個あたり約640〜690msを占めた。

play-serverに、`SURGE_DATA_HOME`で大量のfactory／3rd-party patch directoryの探索を避ける知見があった。editorは保存されたstateで演奏するため、演奏用instanceに全patchの一覧は不要である。一方、初期patchとwavetable等の音声資源は必要であり、空のdata homeで代用してはいけない。

## 決定

### editor内の音源生成

`cat-plugin-player/shared/surge_data_home.rs`（cat-smf-editor から `#[path]` で参照）で、native library・GUI・workerの開始前に、このプロセスだけの`SURGE_DATA_HOME`を設定する。Windowsでは既存の同環境変数、なければ`%ProgramData%/Surge XT`を元データとする。元データがなければ何もしない。

editorは起動後にもSurgeを選べるため、起動曲のpluginを調べた後やlibraryを開いた後に環境変数を書き換えず、この時点で一度だけ準備する。他のpluginのdata homeは変更しない。

`%LOCALAPPDATA%/cat-smf-editor/cache/surge-data-min-v2/<key>/`へ次の構成を作る。v1はpatch一覧だけを最小化した旧cacheであり、上書き・削除せず残す。status・SMF・音色参照の形式は変えず、通常起動でv2を新規生成する。

| 項目 | 最小構成での扱い |
| --- | --- |
| `patches_factory` | `Templates`だけを元directoryへjunctionで接続 |
| `patches_3rdparty` | 空directory |
| `fx_presets`・`wavetables`・`wavetables_3rdparty` | 空directory。一覧構築を避け、FX・wavetable本体は保存state／FXP payloadから復元 |
| その他の最上位directory | 元directoryへjunctionで接続。skins、tuning、modulator preset等を保持 |
| 最上位の通常ファイル | 内容を複製 |
| source marker | 元データのcanonical pathを記録 |

`Templates`は空にしない。Surgeが使うInit Saw等の初期patchを元と同じdirectoryから読み込ませる。play-serverではこれを欠くとFXP復元後も約2dBの出音差が出たという知見がある。editorでもTemplates内にFXPがあることとwavetablesの存在を準備前に検証する。

keyは元のcanonical path、最上位entry名、通常ファイルの内容から作る。patch一覧やwavetable配下を再帰走査しない。再起動では最小構成のentry、空にするdirectory、junction接続先、通常ファイルの内容を検証する。共有directory内容の更新はjunction経由でそのまま見える。source path・構成・通常ファイルが変われば別keyを作る。

新規構成はstaging directoryで検証後にrenameする。準備・検証失敗時は環境変数を変更せず、error.logへ記録して元の構成で起動する。失敗時の掃除は新しく作ったlink・file・空directoryだけを個別に削除し、junction配下を再帰削除しない。既存の破損cacheを勝手に修復・削除しない。

ユーザー／システム環境変数、インストール先、元patch、state cache、SMF、設定形式は変更しない。新規の再生成可能なcacheだけなので、既存データの形式移行は不要である。

### playerの音色一覧とstate生成

editorが内部で作った最小構成を、常駐browserやheadless playerへ継承させない。`resident_link::spawn`と`state_request::headless`の両方で、起動前の`SURGE_DATA_HOME`へ戻す。元が未設定なら子プロセスでも未設定にする。一覧探索・roots解決・必要時のstate生成は従来の全patchを使う。

ユーザーが最初から最小構成を環境変数で明示した場合は、その指定を維持する。editor／play-serverの既知のmarkerを持つ構成は元データと接続先を検証して再利用し、さらに最小化しない。

play-serverと同じ`CMRT_SURGE_MINIMAL_DATA_HOME=0`、`false`、`no`で無効化できる。通常利用でユーザーによる環境変数の設定は不要である。

## patch一覧だけを最小化した段階の実測と検証

2026-10-19、ADR 0011と同じ実曲`concerto-in-e-major-mids/score.mid`、既定音源5個、release buildで比較した。両条件とも同じ新しいexeを使い、`scripts/measure_startup.ps1`で実設定とcacheを隔離先へ複製した。最小構成cacheのjunctionは接続として複製し、元資源の配下はコピーしない。OSのcold boot測定ではない。

| プロセス起動から最初の非無音出力blockまで | 1回目 | 2回目 | 3回目 | 中央値 |
| --- | ---: | ---: | ---: | ---: |
| 最適化無効 | 3374ms | 3360ms | 3363ms | 3363ms |
| 最適化有効・構成cacheあり | 1192ms | 1132ms | 1088ms | 1132ms |

中央値で約2231ms、約66%短縮した。ADR 0011の経路分離前3643msからは約69%短縮である。最小構成cacheなしの初回生成込みは1回測定で1401msだった。

trace上の音源生成は1個約631〜693msから約195〜246msへ減った。構成cacheの検証は有効時1回目で約11msだった。最小statusを先に読み、全status・履歴・表示構築・fontを演奏callback後に始める順序も維持した。

音色設定済みの別曲`modified/ToccataFugue.mid`は変更後の1回測定で1396ms。ADR 0011の1271msは別試行なので、この曲での改善は主張しない。

音声deviceを使わない`startup_trace::native_audio::surge_data_home_audio_probe`で、元のdata homeと最小構成を別プロセスで比較した。既定音色と実cacheのCloud Horn・Inside The Place・Organ 1を各3回、同じ3音・velocity・sample rateでrenderした。全出力は有限・非無音で、wavetableデータを含むOrgan 1は各262144個のfloatが3回とも元構成と完全一致した。他の音色は元構成でも生成ごとに変動するため、波形一致を全音色の保証とはしていない。人間による聴感検証は含まない。

実利用データの通常GUI起動・終了・再起動も2回確認した。startup/status/settingsはbyte一致、SMFはchecksum一致、履歴は通常更新するopened時刻以外の全項目が一致した。元のSurgeデータを変更せず、editor用cacheを追加した。

filesystemテストはTemplates・音声資源・元patchの保全、更新の反映、cache破損／Templates欠損時の失敗、空白・日本語・shell特殊文字を含むpath、子playerへの環境変数復元を確認する。fmt、clippy、release test、release buildを通した。

### 最適化後の生成内訳

続く再計測では、共通hostのinstance生成要求が5音源で約1007ms、直接CLAPの`factory.create_plugin`だけでも約911msだった。測定専用構成でFX preset・wavetablesの両系統を空にするとcreate合計が約909msから約129msになり、この資源に依存する処理が主な候補と分かった。この時点では出音維持と内部関数別の配分が未確認で、省略をアプリへは適用していなかった。条件と全結果は[生成内訳の測定記録](../measurements/surge-creation-20261019.md)にまとめる。

## FX preset・wavetablesも最小化した追加検証と採用

envで最小構成を指定する方法をさらに試し、62件の実利用音色／FX cache、48件の元FXP直接ロード、既定音色を合わせた111ケースを各構成2回ずつ比較した。最後に読んだファイルのpathだけを実際の参照先へcanonicalizeすると、音色パラメータ・wavetable payloadを含む復元後stateがすべて一致した。独立した元構成の再実行でも波形が安定していた42ケースでは、各省略構成の2回とも全sampleが一致した。Init Wavetable等も追加検証した。

同じ新しいexe・既定音源5個の実曲で、v1構成の明示指定とv2構成の通常起動を各3回測った。プロセス起動から非無音出力までの中央値は1106msから280msへ約75%短縮した。v2での各回は280ms・280ms・287msである。通常起動・再起動でstartup/status/settings/SMFと、opened時刻以外の履歴を保全し、旧v1 cacheを残したまま実データ用のv2 cacheを作成した。

この結果をもとに、editorの演奏用data homeには上記v2構成を採用する。browser・roots探索・state生成は元の全資源を持つ環境で行う。未知の外部wavetable参照、全patch、将来のplugin版、聴感まで保証したものではなく、環境変数による無効化も維持する。ランダム位相の音色はbit一致を合格条件にしない。

測定の再現手順・出力比較の限界・raw FXP経路は[音声比較の記録](../measurements/surge-resources-audio-20261019.md)へ記載し、[play-server ADR 0010](../../../clap-mml-play-server/docs/adr/0010-surge-data-home-and-plugin-identity.md)にも最小化方針と成功結果をフィードバックした。server自身の実装や測定値は今回変更していない。

### 実利用での聴感確認とUX評価（2026-10-19）

ユーザーが普段使う曲での聴感確認を完了し、FX preset・wavetablesを含む最小化の効果が発揮され、UXが劇的に改善されたと報告した。上記の機械検証とは別に、採用後のeditorを使ったユーザー評価として記録する。普段使う曲の聴感確認は完了済みである。

### 再測定

```powershell
./scripts/measure_startup.ps1 -Runs 3 -Environment @{CMRT_SURGE_MINIMAL_DATA_HOME='0'}
./scripts/measure_startup.ps1 -Runs 3
```

初回生成と構成cacheありを区別する。結果は各出力先のresults.jsonとtrace.jsonに残す。音声callbackの非無音blockを測っており、スピーカーからの音を外部収録した値ではない。

## 今回採らない変更

instance生成の並列化やhost資源の共有は導入しない。今回の知見で大きな待ちを除けたため、pluginのthread制約や独立性を変える前に、この小さい変更を優先する。UAPMD・共通host・Surge本体へのpatchは追加しない。

v2でも約129ms相当の`create_plugin`処理などは残る。directory形式bundleのcache検証共有など、別の候補は必要な実データで効果を再確認してから扱い、更新検知を省いて高速化しない。
