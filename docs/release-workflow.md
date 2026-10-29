# Windows release の構成と共通化

## 現在の構成

`.github/workflows/release.yml` は毎日01:23 JSTの起動と手動起動を担当し、
`uapmd-windows-release.yml` を呼ぶ。手動起動の `publish=false` はビルド・検査・テスト・ZIP artifact作成まで。
`publish=true` とscheduleは成功後に `v<Cargo.tomlのversion>` のReleaseを置き換える。
起動は同じリポジトリ内で直列化する。GitHubのconcurrencyは実行順を保証せず、待機中のrunが置き換わる場合がある。

共通workflowはアプリ名と版を `Cargo.toml` の `[package]` から取得する。
package名とexe名が一致する、Windows MSVC向けの単一バイナリアプリを対象とする。
リポジトリ依存の設定は `.github/release.json` に集約する。

- `dependencies`: 公開GitHubリポジトリ、解決するref、兄弟checkoutのディレクトリ名。
  各refを一度だけSHAに解決し、ビルドとnotesで同じSHAを使う。editorにはUAPMD、cat-plugin-playerの共有ソース、clap-mml-play-server内の`core-lib`（`cmrt-core`）、clap-mml-render-tui内の`chord`と`midi-filter`、mascot-render-server内の`mascot-render-client`と`mascot-render-protocol`、integration testの実素材としてcat-music-patternsが必要。
  mascot-render-serverも`refs/heads/main`をSHA解決して兄弟checkoutへ取得する。ここではclient/protocol crateを参照し、server本体のexeはビルド・同梱しない。
  素材も`refs/heads/main`を一度SHA解決し、他依存と同じcheckout・notes・再ビルド判定へ渡す。素材はZIPへ同梱しない。
  MML・コード入力はplayerのshared入力UI・変換経路を使い、TUIのchord変換とGit依存のMML変換器を利用する。
  `cmrt-core`はpath identity用に`default-features = false`で参照し、MMLアプリ本体やhost機能はビルドしない。
- `verified_sha`: 任意の、手元で確認した依存版の基準。上流mainを固定する値ではない。
  一致は依存版の一致を意味し、配布するexeの手動確認済みを意味しない。
- `build_script`: アプリrootから呼ぶPowerShellスクリプト。ビルドと固有の検査を担当する。
- `package_files`: exe以外に同梱するファイル。ZIPではbasenameを使うため、同名ファイルを指定しない。
- `notes_dependency_order`: 本文の依存commit一覧の表示順。全依存のpathを一度ずつ指定する。

本文の原文は `.github/release-notes.md`。player側にも同じテンプレートを置き、
アプリ名・版・同梱ファイル・依存commit・非表示の再ビルド判定情報だけを差し替える。
commit一覧は閉じた `details` に入れる。配布buildのadapterで解決済みplayerとのテンプレート一致を確認し、
文言がずれたまま公開しない。両repoのテンプレートは一緒に更新する。

`scripts/release.ps1` はMSVC x64を検出し、VS 2022/2026のgeneratorを `UH_GENERATOR` で共有build driverへ渡す。
既定の `target/shim` のconfigure/buildは `build.rs` から参照するplayerの共有driverだけが担当する。
adapterは二度目のconfigureを追加せず、実行後に環境変数の元の値を復元する。
既存cacheでは共有driverのstampと既存generator/platformを使用する。
共有hostはUAPMDを読む前に[stb revision補修](../../cat-plugin-player/patches/uapmd/stb-revision-pin/README.md)を読み、
rtlogが使うstbを`2c980bb59875b0d32144a71867fbdebb2f77cd20`へ固定する。
取得先overrideや既存cacheのSHA・変更・rootヘッダを取得前に検査し、不一致なら保全・復旧の診断を出して停止する。
既知のclean cacheでは共有driverのstampで必要な再configureを判断し、既存CPM_SOURCE_CACHEを再利用する。
この固定はplayer/editor共通で、依存checkoutの直接編集や別target-dirは行わない。
UAPMD付属patchの二重適用を避けるため、単にReleaseビルドを試す目的で既存cacheを再configureしない。
配布buildはplayerと同じ静的CRT方針とする。workflowのbuild job全体に
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS=-C target-feature=+crt-static` を設定し、testでも同じ方式を使う。
共有driverがこのflagからC++側の `/MT` も選ぶ。adapterはflagが無い配布buildを拒否し、
playerの `scripts/crt_linkage.py` でexeとそのbuildの埋込みhost DLLのVC++/UCRT DLL依存を検査する。
clap-mml-play-serverのcheckoutは`cmrt-core`のソース取得用で、MMLアプリ本体のビルド用ではない。editorのDLLはeditor自身のtargetで生成し、exeに埋め込む。
ZIPにはexeと `package_files` のライセンス類を含め、VC++再頒布可能パッケージは不要。
音源などの外部資源への依存はこのCRT変更で変わらないが、リリース本文には列挙しない。
ローカルは従来どおり `cargo build --release --locked` で動的CRT版を作る。
CRT方式を切り替えて既存の `target/shim` を使い回さない。配布の静的版は新規CI runnerでビルド・検査する。

cat-plugin-playerのrefは共有ソースを含む版を指す必要があり、不足はadapterで明示して停止する。
今回のlocal検証にはplayerの未commit変更を含むため、旧HEADを共有ソースの検証済みSHAとして記録しない。
公開buildではengineが解決した実際のplayer SHAをnotesへ記録する。共有ソースのcommit/pushと実際のRelease公開は別の操作である。

scheduleの再ビルド判定は、公開Releaseのnotesに記録したアプリSHA、依存SHA、共通engine revisionを比較する。
手動起動は必ずビルドする。Releaseの取得が404なら未公開として扱い、認証・通信などのエラーでは停止する。
共通engineを変更するときは `ENGINE_REVISION` も更新する。Rust stableやrunner imageだけの更新は比較対象外なので、
その更新で作り直したい場合は手動起動する。

公開の削除・作成は元workflowと同じ可変tag方式で、ビルドとテストの成功後にだけ行う。
作成APIの失敗などで削除後にReleaseが無くなる可能性は残る。immutable releasesを使うリポジトリにはそのまま適用しない。
実装作業中にRelease公開、commit、pushは実行しない。

## 類似プロジェクトへの再利用

可能。推奨は既存の `cat2151/github-actions` に共通workflowを一本置き、各アプリには起動用workflow、
`.github/release.json`、固有build adapterだけを残す構成。
[GitHubのreusable workflow](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows)で実現できる。
共通workflow内のcheckoutは呼び出し元を取得するため、ここでのgeneric engineはそのまま移設できる。
共通engineに外部スクリプトを置いて呼び出し元checkoutから読もうとする構成は避け、generic処理をworkflow内に収めている。

移設時は呼び出し元の `uses` を次のように変更する。以下は配置後の例であり、現時点で存在する共有workflowではない。

```yaml
jobs:
  release:
    permissions:
      contents: write
    uses: cat2151/github-actions/.github/workflows/uapmd-windows-release.yml@<共有版のcommit SHA>
    with:
      publish: ${{ github.event_name == 'schedule' || inputs.publish == true }}
      skip-unchanged: ${{ github.event_name == 'schedule' }}
```

他リポジトリから呼ぶ際は共有workflowのアクセス設定を確認する。必要な権限は呼び出し元でも与える。
共有版はSHA固定を推奨する。一つの修正が全アプリへ即時波及するのを避け、確認後に参照SHAを更新する。
各アプリでのビルド・音声動作確認は引き続き必要。

playerも同じMML関連の二つのリポジトリを使う。両アプリとも配布版は静的CRTにし、
共有build driverとexe・埋込みDLLのCRT検査を使う。checkoutとpackage構成は各アプリで定める。
UAPMDの補修、plugin固有処理、音声エンジンまで今回の共通workflowに取り込む必要はない。
複数exe、macOS/Linux、private依存、submodule必須の依存、独自ZIPレイアウトは追加設計が必要。

## AI保守と複製の比較

数個の実験アプリで処理が大きく違う間は、AIに差分を追従させる複製も合理的。
ただしAIが修正しても、全コピーの発見、適用、レビュー、各runnerでの検証は毎回必要で、適用漏れは残る。
今回のplayer用workflowをeditorへコピーした際の不整合が、その具体例。

そのため公開・依存SHA解決・再ビルド判定は共通化し、CRTやshimなど本当に違う部分だけadapterで保守する案を推奨する。
共通化の限界を入力の増加で無理に吸収せず、対象アプリが増えて実例ができてから拡張する。
今回はeditor内で共通engineを実装した段階。他リポジトリへの配置と各callerの移行は未実施。

## 検証

検証用にPyYAMLが必要（実際のActions実行では不要）。
`python -m unittest discover -s scripts -p 'test_release_workflow.py'` で、workflow内のPython処理を
mockしたGitHub/git応答で検証する。初回公開、入力一致時の省略、依存変更（`cmrt-core`・`cmrt-chord`・素材の取得元だけの変更を含む）、engine変更、手動再ビルド、
通信失敗、Cargoの全path依存と取得先の整合、mascot-render-serverだけの変更による再ビルド、兄弟checkoutのSHA、ZIPの内容を対象にする。
playerの兄弟checkoutも使い、両workflowが生成する共通本文・uapmd確認文の一致、閉じた折りたたみと非表示markerの位置を検証する。
Actions YAMLはactionlintで検証する。CRT検査の回帰はplayer側の
`python -B -m unittest discover -s scripts -p 'test_crt_linkage.py'` で行う。
素材テストは`cargo test --release --locked --test catalog_and_corpus --test smf_io`で有効なまま確認する。
共有補修の実FetchContent回帰、両アプリのRelease/test、playerの静的CRT検査はそれぞれ必要で、
editorのCRT検査をplayerの検査に代用しない。ローカルの動的CRT版の成功を配布の静的版の成功として扱わない。
GitHub runnerの新規checkoutでの完走は別途必要で、既存ローカルcacheでの成功とは区別する。
共有playerの修正を先にGitHubへ反映し、素材設定を含むeditorを反映してから、明示許可された
両repoの`publish=false`を新規runnerで実行する。run ID、app/依存/stb SHAとbuild/CRT/test/ZIPを確認する。
ZIPの起動・既存SMFの読込と試聴は人間が確認する。実際のRelease公開成功、ZIPとnotesのrevisionは
別の確認であり、非公開runの成功を公開成功として扱わない。commit/push・Actions起動・公開は別途明示許可を要する。

