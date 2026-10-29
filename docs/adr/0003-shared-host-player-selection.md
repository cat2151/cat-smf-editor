# 0003: 共通hostによるtrack設定とplayer選択

## 決定

複数音色のlayer、patch参照による保存、playerによる復元と永続state cacheの次段階の設計は
[ADR 0006](0006-instrument-layers-and-patch-state-cache.md)に記録する。
editorの参照保存・layerモデルを実装し、実browserからの選択・常駐再利用・終了連動、画面編集とcache復元を検証済み。人間による聴取・操作感の確認は別に行う。以下のresident選択の方針を引き継ぐ。

音色・effectの選択だけを常駐するcat-plugin-playerのbrowserへ委譲し、音はeditor自身の音源で鳴らす。再生・範囲・位置・mixerはeditorが持つ。
Rust/native/SMF契約の正本と検証方針は
[player ADR 0029](../../../cat-plugin-player/docs/adr/0029-shared-host-smf-selection.md)に集約する。
独立crateは増やさず、editorのbuildにplayerのApp/UI/MML依存を持ち込まない。

0001/0002のSurge専用host、factory pathによる保存、簡易browserと取消契約は置き換える。
両ADRのそれ以外の試聴・範囲・同pitch・一つの音声時刻・saved原本保護の方針は維持する。
通常SMFのGM Program ChangeやCCをeditorの音源設定として解釈する機能は今回追加しない。

## 操作と安全な反映

tはpatch、xはeffectのbrowserを開く。最初の要求でplayerを`--resident <名前>`で1つ起動して常駐させ、
以降は表示要求だけを送る。名前はeditorのプロセスIDを含み、editorを2つ起動しても混ざらない。
制御は向きごとに1本の名前付きパイプ（1行1つのJSON）、plugin stateは送る側が都度作る共有メモリで渡す。
1本のduplexパイプでは受信待ち中の送信が止まるため2本にした。通信の正本はplayerの
`shared/resident_ipc.rs`で、独立crateは増やさない。

常駐playerは選んだ音色・effectを自分では読み込まず、1 slot分のstateと復元用patch参照をeditorへ送る。
editorは表示要求したときの対象（track・layerの安定ID・曲のsession ID・一括・master・effect欄）と原本パスを覚えておき、
一致する選択だけを対象のmetaへ書いて試聴音源へ反映し、結果をplayerへ返す。
原本、他track、ノート・tempo・未知イベントは変えない。演奏中は現在位置から新しい設定で鳴らし直す。
editorはstateを`Chain::open`で事前検証しない。pluginの生成は1回約0.57 sかかり、高速化の目的に反するため。
形式の検証（`Patch::validate`）は行う。layerは対象trackのtick 0の`instrument-layers`専用meta、masterはconductorの`master-patch-references`専用metaに参照だけを保存する。
cache missにはplayerの`--restore-state`へJSON参照をstdinで渡し、stdoutのstateを受け取る。GUI・音声device・カタログの初期化より前に処理する。browser選択の常駐経路とは寿命を分ける。

Esc・入力に使われないq・×・effect browserを閉じるEnterはbrowserを隠すだけでplayerを終了しない。
editorのパイプが切れるとplayerは終了する。別の終了操作を足さずに、editorを閉じればplayerが残らない。
playerが途中で終了したらeditorはエラーを出し、次の要求で起動し直す。

却下：一時SMFで受け渡して都度playerを起動・終了する方式。起動とplugin読込を毎回待ち（t / xから音まで約0.9 s）、
戻った後もeditorが全trackのpluginを開き直していた。
未導入時はdownload案内を出し、editorの無関係な操作は止めない。fallbackや自動installは追加しない。

## 配布と検証

player sourceとUAPMDを兄弟checkoutに用意する。editor自身のDLLを同梱して読み込み、
インストール済みplayerのDLLを借りない。各repoの既定target/shimとCRT方針を使う。
当初はeditorの配布でも動的CRTを維持したが、userはランタイム導入の案内を減らすため、
配布方法と本文をplayerに揃えると決定した。配布は静的CRT版だけにし、ローカルの動的CRT版は維持する。
共有hostはRustの `crt-static` に従ってC++側も切り替えられるため、独自の切替やUAPMDへの補修は増やさない。
exeと埋込みDLLの両方を検査し、VC++/UCRT DLLに依存する版は公開しない。
本文は両repoで同じ原文を使い、アプリ固有の値だけを差し替え、commit一覧を折りたたむ。
原文の不一致は配布buildを止める。人間向け本文へ内部事情が再び混ざることを防ぐため、
Surge XT・保存した音色の再現・音色やeffectの編集の説明は載せない。
release engineは両依存の解決SHAを記録する。未commitの共有ソースを基準SHAとして偽装しない。
新規GitHub runnerでの完走と、既存cacheでのlocal検証は区別する。
