# editorのpluginインスタンスcache

patch browserの選択は常駐playerから届くが、演奏用インスタンスを所有するのはeditorの
`Audition` / `Voice` / 共通hostの`Chain`。別pluginへの切替で破棄していたchainを、
editorの`Voice`内に非使用中のcacheとして保持する。
UAPMDのgraph・ビルド方式は変更せず、既存の単一音源chainをlayerごとに所有する。
手元のUAPMDソースでは、生成済みインスタンスを貸し出すpool/cacheは見当たらなかった。

## 再利用と音声処理

layerと永続state cacheの設計は[ADR 0006](adr/0006-instrument-layers-and-patch-state-cache.md)を参照する。
本書のcacheは現行の実行中プロセス内のインスタンス保持を指し、終了時にバイト列を保存するstate cacheとは別である。
参照保存・layer対応を実装し、実pluginの独立した発音・停止、実音声出力での往復とcache期限、画面編集・再起動復元を検証済み。人間による聴取・操作感の確認は別に行う。

- cacheはtrack番号とplugin構成ごとに独立。別trackへ貸し出さず、同時に二重使用しない。
- 同一の設定ならstateもprocessorも再生成しない。track内の構成切替では音声streamを作り直す。
- 同じplugin構成で別の音色なら、保持したインスタンスへstateを読み込む。
  reloadに失敗したインスタンスは破棄し、従来どおり新規作成を試す。
- trackの設定グループをcacheし、完全一致では全layerと共通effectをそのまま再利用する。
  一部変更ではtrack内の安定layer IDとplugin identityが一致する音源だけを再利用し、別layer・別trackへ貸し出さない。
  同じpatchの重複layerも個別のchain・Note On/Off状態を持つ。共通effectはlayer合算後に1回だけ通す。
  まだ保持していないeffect構成には新しいchainを作る。異なる構成間で個別effectを移し替える機能は含めない。
- streamのcallbackを停止・破棄した後、保持ノートのNote OffとCC120（All Sound Off）を
  処理してから退避する。退避中は音声callbackに接続せず、main-thread pumpだけ継続する。
  CC120を受け付けないpluginやeffectの残響を汎用的に消す保証はない。
- サンプルレートが変わったときは再利用しない。callbackより先にnativeを破棄しない。

## 保持時間

patch／effect browserが開いている間は1分ごとに最終利用時刻を更新する。
入力の有無、フォーカス、最小化の有無は条件にしない。
browserを開く要求と音色選択の受信でも更新し、閉じたことを検出した時点から5分間保持する。
5分以上使わなければ非使用中のcacheをUI threadで解放し、使用中のchainは残す。
再生開始・停止・track移動だけでは保持時間を延長しない。

表示判定には、editorが起動した常駐playerのプロセスIDとWindowsの可視top-level windowを使う。
residentモードではmain画面を隠し、browserだけを表示する既存の契約を利用する。
playerが表示するウィンドウを増やす場合は、この判定も見直す。

cache数の上限は設けず、保持中はpluginとstateの分だけメモリが増える。
master設定変更、音声エラーなどの既存の音源リセット時と終了時にも解放する。
初回と解放後の再訪では、pluginの生成時間が必要になる。

## ファイル別の演奏準備cache

曲切替とCtrl+Pの試聴移動・キャンセル時は、準備済みの`Audition`の音源を
canonical pathごとにメモリへ保持する。ファイル別cacheには5分の期限を適用せず、
アプリ終了まで保持する。streamは停止指令を送って一時停止し、main-thread pumpは継続する。
再利用時にstreamを再開し、読み直したSMFの音量・音色・effectで演奏する。
master構成が違う場合は破棄して再準備し、track側の変更は既存の構成比較・reload経路で処理する。
キャッシュへ編集内容を保存せず、ファイルは毎回読み直す。永続化形式の変更はない。

## 検証

- `native_session_cache_reuses_the_same_voice_without_sounding_notes`: 音符を送らずに実音声streamの退避・復帰を3往復し、同じvoice・音量・auto gainを確認するignored test。
- `instance_cache_tests`: plugin往復、track分離、stateの再読込、effect構成の区別、所有権の解放。
- `browser_activity::tests`: 開いている間の1分更新、閉じてからの保持期間、再表示。
- `native_round_trips_reuse_chains_and_idle_expiry_keeps_active_audio`:
  インストール済みSurge XT / Vaporizer2と既定音声出力を使うignored test。
  実chainの往復、299秒で保持、300秒でcacheだけ解放し、使用中の音源が残ることを確認する。

```powershell
cargo test --release --locked --bin cat-smf-editor native_round_trips_reuse_chains -- --ignored --nocapture --test-threads=1
```
