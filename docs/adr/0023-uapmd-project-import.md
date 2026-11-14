# ADR 0023: UAPMD projectと既存の演奏モデルをnoteとpatchから接続する

- 状態: 採用
- 決定者: user（取り込みはnoteとpatchを優先。保存は段階的に拡張し、未対応データは省いて今後対応する）
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

## 保存側の目的と段階的な対応

保存の最終目的は、repl-plugin-playerでuapmdzを読み込んだときに同じ内容を演奏できることである。
編集用データをarchiveへ格納するだけでは、この目的を満たさない。
参照したrepl-plugin-player（`84076815d01f2fd6f277b6b46fc701922a74ab6c`）はUAPMDのproject document保存・復元を使い、plugin stateとMIDIクリップをarchiveにまとめる。
取り込み時と同じrevisionのUAPMDはSMFクリップの読み込みとproject writerを提供しているため、それらを使う。

当アプリの演奏展開を再利用し、Song配置やsame-measureなどを展開したMIDIイベントを保存する。
初版はレイヤーを別trackへ分け、移調とdrum pitch routingをMIDIへ反映し、レイヤー音量とtrack音量を合成した。
レイヤーのsoloは元track内で解決してmuteへ反映する。これにより他trackを意図せずsolo対象から外さない。
音源は復元済みのopaque stateを同梱し、patch未指定のレイヤーは既存のdefault instrumentを初期stateで生成する。
編集用SMF、保存先、dirty状態や起動時の復元形式は変更しないため、既存データの移行は不要である。

userは開発途中の初版で一部を削り、別sessionで改善を繰り返すことを選んだ。
この判断により、初版ではtrack/master effect、plugin parameter automation、歌声のaudio、master auto gain、metronome、編集用pattern/scene構造と独自meta eventを保存から省いた。
それらの有無を検出して保存を拒否するロジックは追加しない。
ノート、CC、pitch bend、音源state、レイヤー設定とテンポ・拍子から始め、上記の省略内容を今後対応する課題として残す。
未対応機能を含む曲では同じ演奏にはまだならず、完全再現を達成した扱いにはしない。
演奏時刻をPPQNの整数tickへ戻す際の丸めと、UAPMDのSMF変換がEnd of Trackを演奏イベントへ渡さないことによる末尾休符・clip長の保持も、改善課題に含める。
保存側が作る複数レイヤー・音量・CC等は、取り込み側の初版の制限を越えるため、当アプリ自身への再取り込み拡張も別途必要になる。

## SFZとeffectの保存を優先する

userが初版の保存を試し、現在のdrumに使うSFZとeffectsを次の優先対象に指定した。
SFZも音源のopaque stateとして扱い、別の音源やpresetへ変換しない。
repl側でstateを復元してもdrumが無音になる症状と、generic gainからeffectへの接続が落ちる問題は、受け取り側の[ADR 0015](../../../repl-plugin-player/docs/adr/0015-exported-sfz-and-routed-gains.md)に従って補修する。
stateが外部のSFZやsampleを必要とする場合は、読み込み先にもそのlibraryが必要であり、libraryの同梱・pathの書き換えは行わない。

effectをレイヤーごとのtrackへ複製すると、compressor等がレイヤー合算後の信号を受け取れず音が変わる。
このためtrack effectsを持つ元trackは、各音源とレイヤーgain、合流後のeffect列を1つのDAGとして保存する。
移調・drum routing済みのMIDIをレイヤーごとに異なるUMP groupへ割り当て、UAPMDの既存SMF2 Clip writerで1つのclipへまとめる。
track faderはeffectsの後、master effectsは全trackの合算後に置く。
effectsがないtrackは初版のレイヤー別trackを維持する。

参照したUAPMD revisionのproject schemaはpluginのbypassを保存しないため、bypass中のeffectはstateを同梱したまま接続を迂回する。
再生時の適用有無を保つが、読み込み先のbypassスイッチ自体の復元は保証しない。
UMP group数の制限により、track effectsを共有できるのは1trackあたり16レイヤーまでとする。
それを超える場合は情報を落として保存せず、保存エラーにする。
編集用SMF・cache・起動時復元の形式は引き続き変更せず、既存のuapmdzは再出力で更新できるため、一度限りの移行は不要である。
