# 最小data home適用後のSurge生成内訳（2026-10-19）

## 結論と範囲

この記録は資源単位の切り分け時点の結果である。その後、[音声比較と通常起動の追加検証](surge-resources-audio-20261019.md)を行い、editorへ最小構成v2を採用した。

残る約1秒の待ちは、ほぼSurgeの`factory.create_plugin`にある。資源別の差分実験では、その約86%がFX presetとwavetablesのdirectory内容に依存した。Surge内部の関数単位の時間配分と、この資源を省いても出音を維持できるかは未特定である。

今回変更したのは計測scriptと記録だけで、アプリ、共通host、Surge本体、元データ、通常利用のcache構成は変更していない。音声device・GUIは開かず、note送信・audio renderも行っていない。

関連: [ADR 0012](../adr/0012-surge-minimal-data-home.md)。[集計値と各回の数値](surge-creation-20261019.json)も保存する。

## 条件

- Windows x64、インストール済みSurge XT CLAP、ADR 0012で作成した最小data homeを使った。OS・disk cacheを排出したcold boot測定ではない。
- 新規プロセス内で独立した5音源を順番に生成し、5個を同時に保持してから終了する。各条件3回の合計時間の中央値を比較する。
- 共通hostのrelease版診断テストと、CLAPを直接呼ぶ最小hostの2経路で確認した。直接呼出は任意のhost extensionを提供せず、state loadやgraph生成も含まないため、両経路の差をそのまま特定処理の費用とはしない。
- CLAP ABIはローカルの`uapmd/build/_deps/clap-src/include/clap/`のentry・factory・host・plugin定義と照合した。init・activate・destroyは同じmain threadで行った。
- plugin SHA-256: `6fb598b3759d359ef670e2632ca0d8ff2c19be5389c0ba4801168643d7c5d219`。

## 生成経路の切り分け

| 経路 | 処理 | 5音源分の中央値 | 分かったこと |
| --- | --- | ---: | --- |
| 共通host | 空host生成 | 2.96ms | 支配的ではない |
| 共通host | plugin情報の復元 | 0.35ms | 支配的ではない |
| 共通host | instance生成要求全体 | 1006.59ms | 大半がここに残る。初回のplugin DLL読込等も内包 |
| 共通host | 要求から戻った後のcallback待ち・pump | 約0.03ms | 非同期完了待ちが原因ではない |
| 共通host | graph接続 | 0.32ms | 支配的ではない |
| 共通host | settle | 3.63ms | 支配的ではない |
| 直接CLAP | DLLロード（プロセスに1回） | 37.89ms | instance生成とは分けて計測 |
| 直接CLAP | entry.init・factory取得 | 合計約0.01ms | 支配的ではない |
| 直接CLAP | factory.create_plugin | 910.96ms | hostを介さずとも1個あたり約180msかかる |
| 直接CLAP | plugin.init | 0.11ms | 支配的ではない |
| 直接CLAP | plugin.activate | 0.59ms | 支配的ではない |

直接呼出のcreate合計はwall約911msに対してprocess CPU約906msだった。ただしCPU時間の粒度は約15.6msで、filesystem処理のCPU時間も含む。CPU時間が長いことだけで、I/Oを原因から除外できない。

元の最小構成では、create中に1音源あたり408回のread、約4459回のその他I/O、約489KBのreadを観測した。その他I/Oはprocess I/O counterであり、すべてをstatとは断定しない。

## 資源別の差分実験

元の最小構成をjunctionで参照する測定用treeを別の一時directoryへ作り、指定した資源directoryだけを空にした。Templatesは全条件で保持した。元データを移動・削除せず、通常のeditor用treeも変更していない。

| 測定専用構成 | create合計の中央値 | controlとの差 | read回数／5音源 | その他I/O回数／5音源 |
| --- | ---: | ---: | ---: | ---: |
| 全資源あり（control） | 909.35ms | — | 2040 | 22337 |
| FX presetだけ空 | 498.21ms | −411.15ms | 585 | 12017 |
| factory・3rd-party wavetablesだけ空 | 530.09ms | −379.27ms | 1500 | 11297 |
| factory wavetablesだけ空 | 847.70ms | −61.65ms | 2520 | 20037 |
| 3rd-party wavetablesだけ空 | 664.97ms | −244.38ms | 2040 | 13657 |
| modulator presetだけ空 | 902.56ms | −6.79ms | 2040 | 22337 |
| skinsだけ空 | 900.68ms | −8.67ms | 2040 | 22337 |
| tuningだけ空 | 927.17ms | ＋17.82ms | 2040 | 22337 |
| FX presetと両wavetablesを空 | 129.09ms | −780.27ms | 45 | 977 |
| 全資源あり（実験後のcontrol） | 906.78ms | −2.57ms | 2040 | 22337 |

FX presetには291ファイルあり、これを空にすると1音源あたりのreadが408回から117回へちょうど291回減る。したがってFX preset directoryに依存する読込・一覧構築等は有力な削減対象である。ただし、その内部でread・列挙・parse・allocationがそれぞれ何msかまでは測れていない。

wavetablesはfactoryに190ファイル、3rd-partyに573ファイルあった。この2directoryに依存する処理も大きい。factoryだけを空にするとreadが増える条件があり、代替資源探索等で処理経路が変わる可能性がある。各行の差は排他的な内訳ではなく、足し算して正確な配分とはしない。

modulator・skins・tuningはI/O回数が変わらず、この3回ずつの試行から小さい時間差を改善効果とは判断しない。両系統を空にしても残る約129msの内部処理は未特定である。

## 次の候補と未確認事項

| 順位 | 候補 | 次に必要な確認 |
| --- | --- | --- |
| 1 | FX presetの一覧用readを演奏用instanceから外せるか | 既定音色、保存state、内蔵FX、Surge XT Effectsの出力・復元を元構成と比較。必要資源と一覧専用資源を分けられるか確認 |
| 2 | wavetablesの一覧・metadata準備の重複削減 | wavetable参照・埋込みstate・user追加資源・欠損時fallbackを保持できる方式を調査。空directory化をそのまま採用しない |
| 3 | 残り約129msの関数別profile | OSの詳細I/OトレースはWPRでAccess deniedとなり取得できなかった。追加権限の要求やpluginへのpatchは行わず、この粒度で調査を区切る |

測定用の省略構成で音を比較していないため、約780msを安全に削減できるという保証ではない。アプリの起動→発音時間を再度短縮したという結果でもない。今回はボトルネック候補の特定までを完了した。

## 再実行

64-bit WindowsのPythonから、既存の最小data homeを指定する。出力先は元データの外側の新規directoryを使う。

```powershell
python scripts/probe_surge_resources.py --data-home 'X:/cache/surge-data-min-v1/key' --output 'X:/measurements/surge-new-run'
```

`probe_surge_creation.py`は単独でも実行できる。実行前から設定された`SURGE_DATA_HOME`を使い、phase別のwall／process CPU／I/OをJSONへ残す。`probe_surge_resources.py`は測定専用の子プロセスにだけ各構成を渡し、各回のJSON・logとsummary.jsonを保存する。fixtureは再調査用に残し、自動削除しない。
