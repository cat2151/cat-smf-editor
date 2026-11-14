# ADR 0023: UAPMD projectはnoteとpatchを既存の演奏モデルへ取り込む

- 状態: 採用
- 決定者: user（指定projectのnoteとpatchの演奏を優先し、YAGNIで範囲を絞る）
- 関連: [0006: instrument layer](0006-instrument-layers-and-patch-state-cache.md)、[0007: effect chain](0007-effect-chain-on-uapmd-graph.md)、[0008: patch参照のpath](0008-machine-independent-patch-paths-in-smf.md)

## 背景と決定

repl-plugin-playerで保存したUAPMD projectのnoteとpatchを、このアプリで演奏したい。
指定されたprojectは、開始位置0のMIDIクリップと、各trackに1つの音源、その保存stateを持つ。
参照したUAPMD（`4ec5c3b1c30130fdd3b7d4b375082273247e561f`）にはprojectとMIDI2クリップのreaderがあり、依存するumppiにはMIDI2の基本イベントをMIDI1へ変換する処理がある。
一括でこのアプリのモデルへ変換するAPIがなくても、それはparserの再実装を必要とする理由にはならない。
既存readerと変換処理を使い、結果をこのアプリのnote・tempo・拍子・patch参照へ対応付けるadapterを持つ。

当初の復元元はcatalogのpatchとeffect presetだったが、projectに保存されたstateはそれらから再生成できるとは限らない。
別のpresetへ置き換えると元のpatchを失うため、state本体を解釈せず、内容hashで識別する永続assetとして取り込む。
これは再生成可能なcacheではない。SMFにはhashとplugin識別情報だけを残し、復元時にはassetのchecksumを確認する。
assetの不存在や破損を別の音色へのfallbackで隠さない。
既存のcatalog／effect参照と保存データは変更せず、新しい復元元を追加する。
このassetは別の環境へSMFだけを渡しても復元できないため、移動時にはassetも必要になる。

## graphをモデル拡張の課題として残す

UAPMD projectの任意graphは、現在のinstrument layerと直列effect chainでは忠実に表せない。
userの判断で、汎用graphの復元は将来の当アプリのモデル拡張として扱い、今回のnoteとpatchの演奏へ含めない。
現在の取り込みは、音源1つにMIDI入力とaudio出力を接続する形を確認して既存の演奏経路へ渡す。
複数pluginや意味を再現できない接続を直列へ並べ替えて読み込まない。
分岐・合流・bus routing・master処理を扱う必要が生じた時点で、ADR 0006の汎用graph方針と合わせて保存モデルと演奏モデルを拡張する。

## 引き受ける制限

初版は開始位置0・trimなし・各音源trackにMIDIクリップ1つ・UMP group 0のnoteを扱う。
tempoと拍子はmasterから受け取り、自動生成されたclip名はSMFのtextへ転記しない。
未対応のイベント、配置、音量・mute／solo、graphは読み込み時に拒否し、黙って情報を落とした曲として演奏しない。
汎用importerやMIDI2の全情報保持を先回りして実装せず、需要が生じたときに対応範囲を広げる。
