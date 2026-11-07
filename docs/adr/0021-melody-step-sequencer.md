# ADR 0021: 通常の入れ物のpatternを、行=音高のstep gridで打ち込む

- 状態: 採用
- 決定者: user（drum kitとコードを共有する、行=音高のgrid、1レーン表示とtie印の方式へ後で切り替えられる作り、音の長さを「くっついたグリッド」で表しShift+L／Shift+Hで伸縮する、伸ばした先で同じ音高の次の音の頭に当たったら止まる、1つの音を頭の縦線つきの1本の帯で描く、Shift+Dが入れ物の種類でgridを分ける、`/`で行をChromatic→Scale→Penta→Chordと絞る、keyとscaleはkey signature・chordはchord guideに従う、表示中の小節に出てくる全scale・全chordの構成音を行にする、置いてある音の行は絞り込みの外でも残す、絞り込みを入れ物ごとにローカル設定に保存し初期値はChord、音名のクリックで試聴する、再生中のstepを1本の縦線で示す、左に縦zoomバーを置き入れ物ごとに覚える、keyが無ければC major・chordが無ければpentatonic、置く音のvelocityは100固定、格子にのらない既存の音の扱い、arpeggioは今回作らない）
- 関連: [0017: ドラム編集](0017-drum-sequencer.md)、[0018: Pattern／Scene／Song](0018-pattern-scene-references.md)

## 行=音高と、層の分け方

bass・chord・melodyを打ち込むgridとして、userは行=音高のpiano roll型を選んだ。同じ列の別の行を埋めればchordになるためである。各マスに音名を書く1レーン型（303／tracker型）と、前のstepとつなぐtie印の方式は今回作らないが、後で切り替えられることをuserが条件にした。

この条件を満たすため、3つの層に分け、どの層の公開関数も画面の行番号やキーを引数に取らない。

- score層（`score::step_notes`）: 音の並びと、`(音高, 小節, step)`を受ける編集（置く／消す・伸ばす・縮める）
- 配置層（`view::drums::melody`）: どの音をどのマスに描くか。行の絞り込みもここが持つ
- 操作層（`melody.*`のcommand）: キーからscore層の編集への対応

1レーン型は配置を1つ足すだけ、tie印は「直前の音を1 step伸ばす」へキーを対応付けるだけで済み、score層は変えない。

## Note Offを自分で持つ

drumのNote Offは`score::drums::rebuild`が次の打点か小節末へ付け直すが、旋律では音の長さそのものが編集内容なので、step_notesは各音のNote Offを自分で置き、rebuildに頼らない。step用のmetaは持たず、pattern本体の実noteだけを保存する（0017と同じ）。同じtickに前の音のNote Offと次の音のNote Onが並ぶときは、Note Offを先に置く。逆にすると隣り合う同じ音高の2音で後の音が切れる。

伸ばす操作の止まり先の「patternの範囲の終わり」は、score層がpatternを知らないため、呼び出し側が終わりのtickを渡す。

## 格子にのらない音を隠さない

MMLや録音で入った、step境界にのらない音も、頭の線と帯で描き、色で「格子からずれている」ことを示す（user）。消すと音全体が消え、伸縮すると終わりだけをstep境界へそろえ、頭は動かさない。編集内容を隠しも書き換えもしないという0017と同じ考えである。

## 和声に従う行の絞り込み

128行をそのまま並べると、使う音を探すのにスクロールが要る。userはkey signatureのkey・scaleとchord guideのchordで行を絞ることを選んだ。

和声の取得（`score::harmony`）は曲のtickだけを受け取り、patternや画面を知らない。patternの各stepを曲のtickへ直すのは配置層で、patternの先頭は試聴のtempoと同じ規則（0018）で決める。1レーン型になっても同じAPIを使える。

行は表示中の小節に出てくる全scale・全chordの構成音を合わせたものにし、各列ではその位置の和声に無いマスを暗くする（user）。列ごとに行を変える案は、カーソルを横に動かすたびに行が入れ替わるため採らない。chord guideは拍ごとにchordを置く使い方が普通にあるので、1小節に複数のchordがある前提で作る。

pentatonicとscaleの音程は録音設定の`Key`・`Scale`をそのまま使い、表を二重に持たない。chord guideの無い曲では初期値のChord表示がkeyのpentatonicになる（user）。新しい曲にはC majorのkey signatureが自動で入るので、多くの曲でC major pentatonicの行になる。

絞り込みと縦zoomは入れ物ごとに覚える（user）。

再生中のstepは、drum gridのように列を枠で囲まず、列の中央の1本の縦線で示す（user）。列が表示より高いため、枠は上下の辺が見えず、左右2本の線に見えて混乱する。縦zoomバーは横timelineの縦zoomバーと同じ見た目にし、クリック・ホイールはその位置を中心にzoomし、ドラッグした範囲はちょうど表示の高さになるまでzoom inする（user）。上限は1行が表示の高さになるまでで、固定の上限を置くと狭い範囲のドラッグが範囲に合わない。行の高さを変えるだけで、横timelineのadapterには載せない。gridは時間軸でなく小節内のstepを並べるためである。

行の高さの中央は、音高でなく行の位置で取る。開いたときと絞り込みを切り替えたときに、一番高い音の行から一番低い音の行までの中央を表示の中央へ合わせる（user）。音高の中央へ合わせると、絞り込みで行が間引かれたときに片端が表示範囲から外れる。keyの主音の行の下にoctaveの区切り線を引く（user）。

## いつgridで開くか

Drum・Drum kitの入れ物はpatternに入ると必ずgridで開く。通常の入れ物は、入れ物ごとに「最後にstep gridで編集した」を覚え、覚えている入れ物だけ、候補のクリック・アプリ復帰・Undoでgridを開く（user）。Shift+Dで開くと覚え、Escや窓を閉じると忘れる。sessionに保存し、再起動しても残す。

開くかどうかは`patterns::navigation`の判定1か所で決め、入口ごとに入れ物の種類を調べない。入口ごとに調べると、ある入口だけgridが開かない食い違いが起きる。通常の入れ物も必ずgridで開く案は、piano roll・event list・空のpatternへのMML入力で編集する入れ物で毎回gridを閉じることになるため採らない（user）。
