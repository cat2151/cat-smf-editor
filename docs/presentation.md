# 操作画面と補助窓の表示

`src/view/presentation.rs` が、command の入力所有権と画面の優先順位を対応付ける。
範囲選択menuだけを前面にしたり、Virtual MIDI keyboardだけを隠したりする個別判定は追加しない。

## 表示規則

- `Screen::presentation` は全Screenの表示方針を列挙し、`allows_companions`と画面内の描画層を導く。通常の作業画面と範囲選択中（menuを開く前）は補助窓を併用でき、menu・入力・確認中は操作を載せたviewportを優先する。
- パレットは起点のviewport、独立したcommand入力はroot、Scene/Songの入力はそのdialogのviewport、再生menuは開いたviewportを優先する。
- egui modal、context menu、menu button、popupは共通の生成経路で表示先を登録する。別窓内の操作も対象とする。popup表示中は背後のcommand処理と鍵盤のpad入力を止め、Escを親画面へ漏らさない。同じ窓群の他の窓も退避し、rootの描画末尾でも全登録窓へ最終的な表示判定を送る。
- 退避は非表示で行う。window、session、位置、入力内容を破棄せず、非表示のcallbackは実行しない。閉じる操作・queued inputも通さない。復帰時は元の操作先へfocusを戻し、Windowsが復帰した補助窓をactivateしても入力先を奪わせない。
- ヘルプは現在の操作中に明示的に呼ばれた場合に表示できる。別の操作へ移った際にはこの許可を引き継がない。
- Overviewは補助窓を退避するが、画面内では作業画面の層を保ち、その右クリックmenuを覆わない。
- rootの浮動操作画面はForegroundに置き、入力所有者を同じ層内でも手前にする。背景の操作は無効にする。ドラム・パラメータカーブ内の入力は、その入力を載せる画面で操作できる。
- `layout.toml` でpaneに置いたviewは、載っている窓の一部として扱う。独自の退避判定を持たず、rootのpaneはCentralPanelと一緒に無効になり、dock windowのpaneはその窓と一緒に退避する。focus paneのScreenは通常の作業画面と同じく補助窓を併用でき、menu・入力・確認中はpaneのfocusに関わらずその操作が優先される。
- dock window（`layout.toml` の `[[window]]`）は `Native::Dock` として `Native::show` で出す。構成はtomlが持つので、閉じる操作はアプリの終了になる。
- `layout.toml` はrepoの `target` 配下のexeなら `config/layout.toml` を直接読み、配布exeならAppData内の手動配置ファイルを読む。自動生成しない。変更監視と外部エディタも同じファイルを使う。GUIは標準配置で先に表示し、音声準備中・歌声生成中はレイアウトのファイル探索・読み込み・モニタ列挙を行わず、最初の非ゼロ音声出力後に配置を反映する。再生を停止した場合や再生対象がない場合は音声を待たない。
- dock windowのtitleはview名から自動生成する。`monitor_placement` はモニタの作業領域に対する位置・外寸の百分率で、枠とタイトルバーを含む。`monitor` の `left` / `center` / `right` はモニタ中心X座標順、整数はWindowsのDISPLAY番号。偶数台のcenterや中心Xが同じ候補は番号指定を求め、未接続・不正な設定では直前の有効な配置を維持する。詳細と記述例は `config/layout.toml` に置く。
- 外部窓のview（Output analysis・Song・Scene・Virtual MIDI keyboard・Help）をpaneに置くと、その外部窓は出さない。`modal::begin`・`commands::route`・dialog・再生menu・`modal::end` は載っている窓が行う。view側でも回すとpaletteのキーを食うため。入力のScreenは外部窓のときと同じ。
- 出す条件（analysisのworker・録音session・Sceneの表示）を満たさない間、そのpaneは空のまま残り、focusを持たない。keyboardの付随窓（settings・piano・capture・作成時の選択）とSong/Sceneのdialogは外部窓のまま。
- Help paneはfocusを取らず、focusを持つviewのhelpを出す。置いている間は `F1` で外部窓を開かない。
- step sequencer（Drum sequencer・Drum kit・Step sequencer）をpaneに置くと浮動Windowを出さない。値の入力（`DrumInput`）とtrack作成の確認（`DrumCreate`）の間は、そのpaneの窓が退避せず、そのpaneだけが有効で同じ窓の他paneは無効になる。Kitの Note picker は浮動のまま、そのpaneの窓に出る。
- paneに置いたstep sequencerは、入力先が他のpane（keyboard・mixerなど）へ移っても開いたまま残り、入力先はfocusの枠で示す（user）。入力先をgridへ移すのは、userの操作で開いたとき（Shift+D・候補のクリック・trackの選択）だけで、Undoなどで開き直ったときは入力先を変えない（user）。gridを開くときに閉じるのは浮動の編集画面だけで、paneに置いたviewは閉じない。閉じると、そのpaneにあった入力先がevent listへ飛ぶため。
- Parameter curve（Macro variation curve含む）をpaneに置くと浮動Windowを出さない。値の入力（`ParameterInput`）の間は、そのpaneの窓が退避せず、そのpaneだけが有効になる。variationの値選択は浮動のまま、そのpaneの窓に出る。parameterを選ぶpickerはmain windowに出る。
- Track Overviewをpaneに置くと浮動Windowを出さない。開いている間は、浮動版でmain windowが持つ扱いをそのpaneの窓が持つ: その窓は退避せず、そのpaneだけが有効になる。同じ窓の他paneとmain windowは無効、他のdock window・外部窓は退避する。
- paneに置いたピアノロールのChord timingは浮動のまま、そのピアノロールの窓に出る（`ChordTiming` の間はその窓を優先する）。dock windowもrootの `prepare` と同じく毎frame `presentation::layers` で浮動窓の登録と所有者を始め直す。始め直さないと登録がframeごとに積み上がる。

## 画面を追加する場合

1. `Screen`へ追加し、入力所有権と`presentation`の方針を決める。網羅的matchのため表示規則の追加漏れはcompile errorになる。
2. rootの浮動窓は`presentation::window`、浮動入力は`presentation::area`を使い、Screenと実際の安定IDを渡す。egui modalは`presentation::modal`を使う。context menu・menu button・popupは同moduleの共通関数を使う。
3. ネイティブ窓は`presentation::Native::show`を使う。独自の`show_viewport_immediate`や別窓固有のmodal退避判定を作らない。builderのvisibleは画面自身の開閉条件だけに使う。
   別窓の入力の読み取りとViewportCommandは`presentation::input_for`・`send_to`を通す。eguiは問われたviewport idの状態を作り、それを出していないフレームの出力に載せるので、backendがその窓を開いては閉じ続ける。通すのはこのフレームか前のフレームに`Native::show`で出した窓だけで、まだ出していない窓への命令はフレームの終わりに、そのフレームに出したときだけ届く。共通テストが`input_for(`・`send_viewport_cmd_to(`の直呼びを落とす。
4. `palette/tests/windows.rs`へ画面・窓のfixtureを追加する。presentationの共通テストはそのfixtureを使い、全root操作画面と全別窓種別の退避を確認する。
5. root描画の`prepare`・`refresh`・`finish`を維持する。別窓の表示後に開いた操作も、`finish`で最終判定へ反映される。
6. paneに置けるviewを足すときは、`ViewKind`・`score_panes`のleaf・`config/layout.toml`の画面名一覧に足す。一覧はserdeが受け付ける名前と突き合わせるテスト（`the_template_lists_every_view_name`）があり、全viewを1つずつdock windowに置いて描画・focus・退避を確かめるテスト（`every_view`）は網羅的matchでそのviewを開く準備を求める。

鍵盤は表示と音声進行を分けている。退避時には押下中のpadを解放し、入力を止める一方、録音clock・capture・phraseは進行する。再表示で録音sessionを作り直さない。
楽曲・sessionの保存形式と復元形式は変更していない。

## 検証

`cargo test --bin cat-smf-editor presentation`、`cargo test --bin cat-smf-editor suspension`と全体テストで、画面所有権、別窓の表示遷移、遅れて開く別窓内modal、非表示窓への入力、背景click、paneの無効化、録音の保持を確認する。`cargo test --bin cat-smf-editor every_view` で、全viewをdock windowに置いた描画・focus・退避を確認する。`cargo test --bin cat-smf-editor settles` で、全viewを本体・dock window・自分の別窓に置いて30フレーム回し、各フレームのoutputに描いていない別窓が出ないこと（出るとbackendが既定題名の窓を開く）と、窓とViewportCommandの数が増えないことを確認する。
