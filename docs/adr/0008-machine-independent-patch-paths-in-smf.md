# ADR 0008: SMFにはマシン依存の絶対pathを書かず、rootはアプリがcacheで持つ

- 状態: 採用
- 関連: [0006: instrument layerとstate cache](0006-instrument-layers-and-patch-state-cache.md)、参照型は`cat-plugin-player/shared/patch_reference.rs`

## 背景

SMFは最終的に公開する成果物であり、手元にだけ置くcache・settingsとは扱いが違う。
現在の`PatchReference`は、plugin bundleの`bundle_path`と、patch・presetの`source.path`・`source.file`を絶対pathのままJSONへ書く。
このためSMFに次の問題がある。

- user名を含む`C:\Users\<user>\...`や、手元のdrive構成（sfzの置き場など）が公開物に載る。
- pluginやpatchのinstall場所はuserごとに違い、同じuserでもsfzなどの置き場は移す。絶対pathは移動・別環境で即座に復元不能になる。
- `\`区切りはUnixで読むと1つのファイル名になり失敗する。JSONでは`\\`にescapeされ、1文字につき1byte多い。

## 決定

### SMFに書くもの

- pluginはformat・ID・name・vendorで識別する。`bundle_path`はSMFに書かない。
- patch・presetのpathは、そのpluginのpatch／preset rootからの相対pathで書く。Dexedの論理path（cartridge + program）と元cartridgeのpathも、どちらも相対pathにする。
- SMF内のpath区切りは常に`/`とする。OSのpathとSMFの相対pathの相互変換は1つの関数に集約し、書出しと読込の両方でそれを通す。
- SMFの相対pathは、空・絶対path・drive指定・`..`・`\`を含むものを読込エラーにする。
  ただしeffect presetの空pathはroot自身を指す。Dragonfly・Shuのようにpresetが本体に組み込まれたpluginは、本体の`.clap`がrootになるため。

### rootはアプリ側が持つ

- CLAP pluginの置き場と、pluginごとのpatch／effect presetのroot（絶対path）は、アプリがpluginごとに検索し、cacheへ永続化して保持する。SMFにもsettingsにも書かない。
- root候補の検索は既存の共通core（cmrtの`patches_dirs`、Vaporizer2・TyrellN6・Floeの本体設定などの解釈）を使い、editorにplugin別のroot解釈を複製しない。
  root表を作るのはplayerの`--patch-roots`で、材料はbrowserと同じplugin list cache・cmrtのpatch catalogの`PatchBase`（`Shared`はその1つ、`PerRoot`は各root、`None`はcatalog上のpluginの`dirs`）・built-in effectの`preset_root`。browserが差し出すpathが必ずどれかのroot配下に入るようにするため。
- editorはその出力をcache directoryの`patch-roots-v1.json`へ保存する。cacheで解けないときだけplayerを起動して作り直し、1回のlookupで何件外れても起動は1回にする。cacheが当たる限り起動が速いまま保てる（[0006](0006-instrument-layers-and-patch-state-cache.md)）。
- 復元時はplugin識別でcache上のrootを引き、相対pathを連結して実ファイルを特定する。
  rootにファイルが無い、または同じ相対pathが複数のrootに見つかる場合は復元エラーにし、別のpatchを黙って選ばない。
- cacheは再生成可能なデータとする。rootを移した場合は再検索でcacheだけが変わり、SMFは書き換えない。
- [0006](0006-instrument-layers-and-patch-state-cache.md)のstate cacheはマシン内のデータなので、解決後の絶対pathを無効化判定に使ってよい。

### 置き場所

- `PatchReference`はplayerとeditorが共有する`cat-plugin-player/shared/`にあるため、相対path化・`/`変換・検証もそこへ置く。editorとplayerで別々に実装しない。

## 却下した案

- 絶対pathを残し、見つからないときだけ相対pathで探す: 公開物に個人情報が残り、互換層が恒久化する。
- `%LOCALAPPDATA%`などの環境変数付きpathで書く: Windows依存のままで、Unixで読めず、手元のdirectory構成も漏れる。
- 表示名だけで探し直す: 同名patchを区別できない（0006で却下済み）。

## 結果

- 同じSMFを、別のuser・別のOS・置き場を移した後の手元で、root cacheさえ揃えば復元できる。
- 作者の既存SMFは絶対pathを含むため、実装時にバックアップを取ってから一度だけ書き換える。旧形式の読込経路は残さない。
- state cacheのkeyは「SMF形式から解決し直した参照」になるので、旧形式で記憶したstate cacheは一度だけ作り直しになる。
