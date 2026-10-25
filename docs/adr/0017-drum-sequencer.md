# ADR 0017: GMノートを保ち、4行を独立した音源へ送るドラム編集

- 状態: 採用
- 決定者: user（Surge XT、4行、16step、GM番号と再生先番号の分離、既存catalog分類の優先）
- 関連: [0006](0006-instrument-layers-and-patch-state-cache.md)

## 目的と判断

小さいドラム編集から始めるため、userはKick・Snare・Hi-Hat・Percussionの4行と、16分音符16stepのmatrixを指定した。16stepごとに前後へ移動し、GM番号は36・38・42・39、各行の再生先は初期値60の編集可能な番号とする判断をuserが選んだ。当初は拍子によらず四分音符4拍分を1ページとした。その後、userは小節番号表示と表示小節のA-B repeatを求め、4/4以外も実際の小節・拍子に合わせる判断を選んだ。表示と再生範囲の意味を一致させるため、現在は1ページを実際の1小節とし、16分音符のstep数を拍子から求める。

既存instrument layerは独立したinstanceと音色参照を復元できるが、全ノートを全instanceへ配送する。音色管理はそれを共有し、ドラムtrackでは行に対応するGM番号のNote OnとNote Offだけを指定先番号へ変換する。通常layerの配送は変えない。SMFのノート自体を音色固有の番号へ書き換えるとGMとしての意味が失われるため、変換は再生時に限る。

音色をPercussionフォルダーから探す案に対し、userは既存catalogのkick／snare／hi-hat／percussion分類を優先するよう補足した。TUIの共通分類はcatalogのカテゴリと表示名を使い、具体的な部位をPercussionより先に排他的に分類している。その処理をcrateとして再利用し、Surge XTに絞った候補を使う。editor独自のキーワード表やフォルダー分類は設けない。最初の対象をSurge XTに限定することと、分類規則をSurge専用にすることは分ける。Percussion行からは、実測で音高がnoteについてくる音色（共有表`patch-pitch-tracking.json`でtrue）を外す。patch browserはそれらをleadに並べるため、外さないと行の初期音色とbrowserの欄が食い違う。照合はcat-plugin-playerと同じ`cmrt_patches::pitch_tracking`を使い、表が取れなければ外さない（[cat-plugin-player ADR 0040](../../../cat-plugin-player/docs/adr/0040-measured-pitched-percussion-categories.md)）。

userはEnterとhjklの往復では入力効率が悪いと報告し、左手のキーとSpaceをnote toggleへ優先するよう求めた。行はKickを最下段にし、4stepの区切りを視覚的に示す。音色の比較も編集の反復に含まれるため、独自の候補dialogを廃止して通常のpatch browserを行の分類で開き、選択中もSolo／全trackのA-B repeatを維持する。

## 保存と既存データ

音色参照は既存のinstrument layer metaへ、4行とlayerの対応・再生先番号は同じtrackのtick 0専用metaへ保存する。step状態を別のpatternとして重複保存せず、通常のGMノートを編集対象にする。既存のstate cacheと再起動時の復元経路を共有し、catalogは音色選択と新規作成にだけ必要とする。

導入時の起動対象SMFとローカル設定の読込経路には、新しいdrum metaが存在しなかった。既存schemaを置き換えず、metaのない曲は通常layerとして扱うため、一度限りの移行は不要である。ドラム作成は独立trackへの追加を基本とし、既存の旋律や音色を置き換えない。当初の固定durationは、次のnoteまたは小節末尾まで伸ばすというuserの指定で置き換える。既存noteの発音位置・velocity・音色参照は保ち、Note OffをUndo可能な編集として再計算する。継続利用中の起動対象SMFには既にdrum metaがあるが、保存schemaと行の識別順は変えないため形式移行は不要である。追加時のvelocityは100とし、4種類以外のGM番号や複数パターン管理は追加しない。
