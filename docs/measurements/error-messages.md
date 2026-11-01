# 画面に出るエラー文の一覧

`src/` の非テストコードで、エラー文脈（`Err(`・`ok_or(`・`ok_or_else(`・`map_err(`・`Error::other(`・`error = Some(`・`errors.push(`・`io::Error::new(`）の直後にある英語の文字列リテラルを正規表現で拾い、画面へ流れない物を除いた一覧。同じ行を2つの文脈が拾った重複は1か所に数える。使用箇所の行番号は一覧を作った時点の物。

## 件数

| 項目 | 件数 |
|---|---:|
| 抽出スクリプトが出した文 | 522 |
| 画面に出ない使用箇所しか無い文（下の「除外した使用箇所」） | 23 |
| 画面に出るが訳す語が無い書式だけの文 | 9 |
| 下の表の行数（集める + その場で訳す） | 490 |
| うち集める（画面に出る使用箇所が2か所以上） | 82 |
| うちその場で訳す（1か所） | 408 |

検算: 522 − 23 − 9 = 490。抽出スクリプトが拾わなかった文は下の「抽出スクリプトが拾わなかった文」に別に置く。

## 画面に出るかの判定

既定は「画面に出る」。`Result<_, String>` などのエラーは呼び出し元で `load_error`・`workspace.error`・`saved.error`・`voicevox.error`・player の status などへ渡るため。次の流れ先だけを除外した。

| 区分 | 流れ先 |
|---|---|
| ログのみ | 起動引数（`src/store/cli.rs`）: `src/main.rs:34` で stderr と `error.log` へ出して終了する |
| ログのみ | 起動前の失敗（`src/main.rs:39`）: `eframe::Error` として `src/main.rs:20` の `error.log` へ |
| ログのみ | window 設定（`src/main.rs:54`・`src/store/settings.rs`）: `settings_error` は `src/store/error_log.rs:54` で `error.log` へ書くだけで、画面のコードは読まない |
| ログのみ | mascot（`src/play/mascot.rs`・`src/play/mascot/`）: `src/play/mascot.rs:28` で `error.log` へ書くだけ |
| ログのみ | `error.log` 自体が書けないとき（`src/store/error_log.rs:24`）: `eprintln!` |
| テスト専用 | `#[cfg(test)]` の module（`src/play/audition_probe.rs`・`src/startup_trace/native_audio/`・`src/view/commands/boundary.rs`）。抽出スクリプトはファイル名に `test` を含まないので拾う |
| 誤検出 | エラー文脈の近くにあるだけでエラー文ではない文字列（`error.log` の分類名・環境変数名・path の要素・窓の見出し） |
| 書式だけ | `{…}`・固有名詞（`VOICEVOX`・`cat-plugin-player`）・API path（`/singers` など）を除くと英字が2文字続かない文（例: `{}: {error}`・`VOICEVOX {path}: {e}`）。訳す語が無い |

`.ok()` や `let _ =` で捨てられて画面に出ないエラーは区別していない（呼び出し元ごとの追跡はしていない）。

## 集めた文の訳（`src/message.rs`）

`{…}` は埋め込む値。`Master: {error}`・`Track {number}: {error}` は画面の用語を前置きにした書式なので英語のまま。

| 原文 | 訳 |
|---|---|
| `Arrangement input target changed` | `配置の入力先が変わりました` |
| `Audio device did not start playback` | `音声デバイスが再生を始めませんでした` |
| `A tempo curve requires SMF format 1` | `tempo curveにはSMF format 1が必要です` |
| `Backup directory is unavailable` | `バックアップフォルダを使えません` |
| `Drum step overflow` | `drum stepが上限を超えました` |
| `Edit the pattern from its destination track` | `patternは配置先のtrackで編集してください` |
| `Invalid destination lane` | `移動先のlaneが不正です` |
| `Invalid drum row` | `drumの行が不正です` |
| `Invalid event` | `eventが不正です` |
| `Invalid Reference Tempo BPM` | `Reference TempoのBPMが不正です` |
| `Invalid scene measure` | `sceneの小節が不正です` |
| `Invalid track` | `trackが不正です` |
| `Invalid track volume` | `trackの音量が不正です` |
| `Layer was removed` | `layerが削除されました` |
| `Master effects editing requires SMF format 1` | `Master effectsの編集にはSMF format 1が必要です` |
| `Measure overflow` | `小節が上限を超えました` |
| `MIDI note must be 0–127` | `MIDI noteは0–127で指定します` |
| `Missing container` | `containerが見つかりません` |
| `Missing container sound` | `container soundが見つかりません` |
| `Missing definition` | `定義が見つかりません` |
| `Missing edit scene` | `編集中のsceneが見つかりません` |
| `Missing instrument layer` | `instrument layerが見つかりません` |
| `Missing measure end` | `小節の終わりが見つかりません` |
| `Missing measure map` | `小節の対応表が見つかりません` |
| `Missing measure start` | `小節の始まりが見つかりません` |
| `Missing pattern` | `patternが見つかりません` |
| `Missing pattern meter` | `patternの拍子が見つかりません` |
| `Missing placement` | `配置が見つかりません` |
| `Missing scene` | `sceneが見つかりません` |
| `Missing scene pattern` | `sceneのpatternが見つかりません` |
| `Missing Song meter` | `Songの拍子が見つかりません` |
| `Missing sound container` | `sound containerが見つかりません` |
| `Missing track` | `trackが見つかりません` |
| `Note crosses measure boundary` | `noteが小節をまたいでいます` |
| `Not a Note On` | `Note Onではありません` |
| `Not a parameter track` | `parameter trackではありません` |
| `No audio output device` | `音声出力デバイスがありません` |
| `No drum settings` | `drum設定がありません` |
| `No notes` | `noteがありません` |
| `No SMF is loaded` | `SMFが開かれていません` |
| `No song` | `Songがありません` |
| `No such track` | `そのtrackはありません` |
| `Parameter control must be finite` | `parameterの値は有限の数で指定します` |
| `Parameter disappeared` | `parameterが無くなりました` |
| `Parameter instrument unavailable` | `parameterのinstrumentを使えません` |
| `Parameter owner changed` | `parameterの持ち主が変わりました` |
| `Parameter value is outside its native range` | `parameterの値が本来の範囲の外です` |
| `Pattern edit target no longer exists` | `patternの編集対象が無くなりました` |
| `Pattern range overflow` | `patternの範囲が上限を超えました` |
| `Pattern requires PPQN timing` | `patternにはPPQNの時間単位が必要です` |
| `Recording exceeds the pattern measure limit` | `録音がpatternの小節数の上限を超えました` |
| `Recording is empty` | `録音が空です` |
| `Recording requires a positive tempo` | `録音には正のtempoが必要です` |
| `Recording requires PPQN timing` | `録音にはPPQNの時間単位が必要です` |
| `Recording time overflow` | `録音時間が上限を超えました` |
| `Reference Tempo requires format 1 track 0` | `Reference Tempoにはformat 1のtrack 0が必要です` |
| `Same measure requires PPQN timing` | `same measureにはPPQNの時間単位が必要です` |
| `Scene length overflow` | `sceneの長さが上限を超えました` |
| `Select a container` | `containerを選んでください` |
| `Select a pattern or scene` | `patternかsceneを選んでください` |
| `Select a scene` | `sceneを選んでください` |
| `Select the chord guide track` | `chord guide trackを選んでください` |
| `Storage overflow` | `保存領域が上限を超えました` |
| `The duration must be at least 1 tick` | `長さは1 tick以上で指定します` |
| `This note has no Note Off` | `このnoteにはNote Offがありません` |
| `This patch has no routed macros` | `このpatchにはrouteされたmacroがありません` |
| `Tick overflow` | `tickが上限を超えました` |
| `Too many SMF tracks` | `SMFのtrackが多すぎます` |
| `Track was removed` | `trackが削除されました` |
| `Trill interval is shorter than the available tick resolution` | `trillの間隔がtickの分解能より短くなります` |
| `Unknown trill` | `不明なtrillです` |
| `VOICEVOX engine is not set` | `VOICEVOX engineが設定されていません` |
| `Audio: {error}` | `音声: {error}` |
| `Audio automation refresh: {error}` | `音声のautomation更新: {error}` |
| `Automation: {error}` | `automationエラー: {error}` |
| `Cannot inspect {instrument} macro routing` | `{instrument}のmacro routingを調べられません` |
| `Cannot read {path}: {error}` | `{path}を読めません: {error}` |
| `Cannot scan {path}: {error}` | `{path}の中を一覧できません: {error}` |
| `invalid ({error})` | `不正 ({error})` |
| `Master: {error}` | `Master: {error}` |
| `Track {number}: {error}` | `Track {number}: {error}` |
| `Unsupported audio format: {format:?}` | `対応していない音声形式です: {format:?}` |
| `Cannot open recording instrument` | `録音用のinstrumentを開けません` |
| `invalid` | `不正` |
| `Parameter instance changed; reopen Automation` | `parameterのinstanceが変わりました。Automationを開き直してください` |
| `Parameter target changed; reopen Automation` | `parameterの対象が変わりました。Automationを開き直してください` |
| `Cannot save {path}: {error}` | `{path}を保存できません: {error}` |

## 集める（画面に出る使用箇所が2か所以上）

| 文 | 使用箇所数 | 区分 | 使用箇所 |
|---|---:|---|---|
| `No SMF is loaded` | 71 | 集める | src/app.rs:162, src/app.rs:234, src/app/creation.rs:44, src/app/creation.rs:88, src/app/parameter_automation.rs:10, src/app/undo.rs:51, src/app/undo.rs:55, src/app/undo.rs:88, src/play/arrangement.rs:19, src/play/arrangement.rs:31, src/play/arrangement.rs:46, src/play/arrangement.rs:118, src/play/automation.rs:100, src/play/automation/pattern.rs:74, src/play/automation/performance.rs:8, src/play/mixer/volume.rs:9, src/player/resident.rs:101, src/player/resident.rs:112, src/player/resident_link.rs:86, src/player/selection.rs:155, src/player/selection.rs:191, src/store/auto_save.rs:69, src/view/effects.rs:79, src/view/macro_editor.rs:98, src/view/note_edit.rs:104, src/view/parameter_editor.rs:150, src/view/recording.rs:215, src/view/trill_edit.rs:21, src/view/trill_edit.rs:32, src/view/trill_edit.rs:158, src/view/commands/execution.rs:91, src/view/command_input/insert_event.rs:23, src/view/command_input/insert_event.rs:32, src/view/command_input/pattern_args.rs:52, src/view/command_input/pattern_args.rs:75, src/view/command_input/pattern_args.rs:140, src/view/command_input/pattern_args.rs:160, src/view/command_input/pattern_args.rs:183, src/view/command_input/same_measure.rs:15, src/view/drums/pattern.rs:24, src/view/files/performance.rs:29, src/view/macro_editor/playback.rs:21, src/view/macro_editor/preview.rs:21, src/view/mml_input/insertion.rs:6, src/view/mml_input/insertion.rs:59, src/view/parameter_editor/persistence.rs:7, src/view/parameter_editor/persistence.rs:15, src/view/parameter_editor/persistence.rs:46, src/view/parameter_editor/preview.rs:63, src/view/parameter_editor/variation.rs:40, src/view/patterns/first.rs:22, src/view/patterns/library.rs:30, src/view/patterns/library.rs:46, src/view/patterns/library.rs:75, src/view/patterns/library.rs:93, src/view/patterns/matrix.rs:5, src/view/patterns/matrix.rs:31, src/view/patterns/membership.rs:4, src/view/patterns/navigation.rs:68, src/view/patterns/navigation.rs:125, src/view/patterns/preview.rs:9, src/view/patterns/preview.rs:38, src/view/patterns/workspace.rs:183, src/view/patterns/workspace.rs:206, src/view/patterns/workspace/dialog.rs:70, src/view/recent/prefetch.rs:202, src/view/recording/capture.rs:62, src/view/recording/capture.rs:141, src/view/recording/take.rs:9, src/view/tempo_curve/trills.rs:29, src/view/voicevox/lyrics.rs:74 |
| `Missing pattern` | 18 | 集める | src/score/pattern/automation.rs:114, src/score/pattern/operations.rs:55, src/score/pattern/operations.rs:130, src/score/pattern/operations.rs:146, src/score/pattern/operations.rs:191, src/score/scene/arrangement.rs:34, src/score/scene/arrangement/occurrences.rs:59, src/view/parameter_editor.rs:308, src/view/command_input/pattern_args.rs:147, src/view/command_input/pattern_args.rs:151, src/view/command_input/pattern_args.rs:159, src/view/command_input/pattern_args.rs:168, src/view/command_input/pattern_args.rs:187, src/view/parameter_editor/persistence.rs:9, src/view/parameter_editor/persistence.rs:52, src/view/patterns/membership.rs:6, src/view/patterns/navigation.rs:70, src/view/patterns/preview.rs:41 |
| `Missing scene` | 12 | 集める | src/score/scene/arrangement.rs:40, src/score/scene/arrangement.rs:87, src/score/scene/arrangement.rs:154, src/score/scene/arrangement.rs:224, src/score/scene/arrangement.rs:284, src/score/scene/arrangement.rs:318, src/score/scene/arrangement.rs:377, src/score/scene/arrangement.rs:393, src/score/scene/arrangement/occurrences.rs:28, src/score/scene/arrangement/occurrences.rs:46, src/view/patterns/workspace.rs:185, src/view/patterns/workspace/dialog.rs:85 |
| `Invalid track` | 6 | 集める | src/edit/event_range.rs:33, src/edit/mml.rs:43, src/edit/mml.rs:68, src/edit/mml.rs:184, src/view/command_input/insert_event.rs:34, src/view/voicevox/combined.rs:60 |
| `No such track` | 6 | 集める | src/edit/lyric/measure.rs:47, src/player/resident.rs:122, src/player/resident.rs:133, src/player/resident.rs:140, src/view/effects.rs:80, src/view/voicevox/singer.rs:203 |
| `Measure overflow` | 5 | 集める | src/score/same_measure.rs:58, src/score/drums/measures.rs:26, src/score/drums/measures.rs:153, src/score/same_measure/insert.rs:18, src/view/command_input/same_measure.rs:25 |
| `Missing measure map` | 5 | 集める | src/score/drums/measures.rs:85, src/view/macro_editor.rs:121, src/view/drums/playback.rs:7, src/view/drums/playback.rs:22, src/view/parameter_editor/preview.rs:71 |
| `No song` | 5 | 集める | src/view/drums.rs:59, src/view/drums.rs:114, src/view/drums.rs:194, src/view/drums/playback.rs:5, src/view/drums/playback.rs:21 |
| `Not a Note On` | 5 | 集める | src/edit/lyric.rs:31, src/edit/note_length.rs:12, src/edit/note_move.rs:13, src/edit/note_start.rs:11, src/edit/lyric/measure.rs:89 |
| `Storage overflow` | 5 | 集める | src/score/pattern/operations.rs:37, src/score/pattern/operations.rs:63, src/score/pattern/operations.rs:165, src/score/pattern/recording/capture.rs:368, src/view/patterns/membership.rs:29 |
| `Cannot read {}: {error}` | 4 | 集める | src/store/config.rs:192, src/store/config.rs:196, src/view/selection_history.rs:22, src/view/selection_history.rs:31 |
| `Drum step overflow` | 4 | 集める | src/score/drums.rs:143, src/score/drums/measures.rs:47, src/score/drums/measures.rs:50, src/score/drums/measures.rs:51 |
| `Edit the pattern from its destination track` | 4 | 集める | src/app.rs:165, src/app.rs:236, src/view/note_edit.rs:81, src/view/mml_input/insertion.rs:54 |
| `Missing Song meter` | 4 | 集める | src/score/scene/arrangement.rs:104, src/score/scene/arrangement.rs:132, src/score/scene/arrangement.rs:381, src/score/scene/sizing.rs:18 |
| `Missing placement` | 4 | 集める | src/score/scene/arrangement.rs:293, src/score/scene/arrangement.rs:304, src/score/scene/arrangement.rs:397, src/score/scene/arrangement.rs:404 |
| `Select the chord guide track` | 4 | 集める | src/view/command_input.rs:157, src/view/command_input/guide_edit.rs:59, src/view/command_input/guide_edit.rs:97, src/view/command_input/insert_event.rs:26 |
| `Tick overflow` | 4 | 集める | src/edit/mml.rs:143, src/edit/mml.rs:144, src/edit/mml.rs:194, src/edit/mml.rs:195 |
| `Too many SMF tracks` | 4 | 集める | src/edit/tracks.rs:95, src/edit/parameter_curve/metadata.rs:155, src/score/chord_guide.rs:60, src/view/drums.rs:62 |
| `Audio device did not start playback` | 3 | 集める | src/app/startup.rs:96, src/app/startup.rs:156, src/view/opening.rs:261 |
| `Automation: {error}` | 3 | 集める | src/play/automation.rs:215, src/play/automation.rs:248, src/play/transport.rs:194 |
| `Backup directory is unavailable` | 3 | 集める | src/store/saved.rs:127, src/store/saved.rs:200, src/view/files/performance.rs:47 |
| `Missing container` | 3 | 集める | src/score/pattern/automation.rs:44, src/score/pattern/automation.rs:161, src/score/pattern/operations.rs:151 |
| `Missing definition` | 3 | 集める | src/score/pattern/operations.rs:107, src/score/pattern/operations.rs:109, src/score/pattern/operations.rs:216 |
| `Not a parameter track` | 3 | 集める | src/edit/parameter_curve/metadata.rs:192, src/score/automation.rs:38, src/view/parameter_editor/persistence.rs:18 |
| `Parameter instrument unavailable` | 3 | 集める | src/play/audition_parameters.rs:23, src/play/audition_parameters.rs:29, src/play/audition_parameters.rs:46 |
| `Recording time overflow` | 3 | 集める | src/score/pattern/recording.rs:66, src/score/pattern/recording/capture.rs:139, src/score/pattern/recording/recorder.rs:169 |
| `Same measure requires PPQN timing` | 3 | 集める | src/score/same_measure.rs:144, src/score/same_measure/insert.rs:24, src/view/command_input/same_measure.rs:19 |
| `Scene length overflow` | 3 | 集める | src/score/scene.rs:81, src/score/scene.rs:86, src/score/scene.rs:105 |
| `This note has no Note Off` | 3 | 集める | src/edit/note_length.rs:17, src/edit/note_move.rs:15, src/edit/note_start.rs:13 |
| `Track {}: {e}` | 3 | 集める | src/play/audition_offline.rs:56, src/play/audition_output.rs:61, src/view/opening.rs:175 |
| `Unknown trill` | 3 | 集める | src/edit/trill.rs:292, src/edit/trill.rs:324, src/edit/trill.rs:343 |
| `invalid ({error})` | 3 | 集める | src/score/event_content/sequencer.rs:223, src/score/event_content/sequencer.rs:252, src/score/event_content/sequencer.rs:262 |
| `A tempo curve requires SMF format 1` | 2 | 集める | src/edit/tempo_curve.rs:120, src/edit/tempo_curve.rs:177 |
| `Arrangement input target changed` | 2 | 集める | src/view/patterns/workspace/dialog.rs:65, src/view/patterns/workspace/dialog.rs:108 |
| `Audio automation refresh: {error}` | 2 | 集める | src/play/audition_pump.rs:37, src/play/audition_pump.rs:52 |
| `Audio: {error}` | 2 | 集める | src/play/audition.rs:125, src/play/audition.rs:206 |
| `Cannot inspect {} macro routing` | 2 | 集める | src/view/macro_editor.rs:116, src/view/macro_editor/preview.rs:34 |
| `Cannot scan {}: {e}` | 2 | 集める | src/score/catalog.rs:15, src/score/catalog.rs:17 |
| `Invalid Reference Tempo BPM` | 2 | 集める | src/edit/reference_tempo.rs:48, src/edit/reference_tempo.rs:118 |
| `Invalid destination lane` | 2 | 集める | src/score/scene/arrangement.rs:286, src/score/scene/arrangement.rs:298 |
| `Invalid drum row` | 2 | 集める | src/score/drums.rs:141, src/score/drums/measures.rs:91 |
| `Invalid event` | 2 | 集める | src/view/command_input/guide_edit.rs:64, src/view/command_input/guide_edit.rs:103 |
| `Invalid scene measure` | 2 | 集める | src/score/scene.rs:72, src/score/scene.rs:73 |
| `Invalid track volume` | 2 | 集める | src/play/track_volume.rs:30, src/play/mixer/volume.rs:15 |
| `Layer was removed` | 2 | 集める | src/player/resident.rs:127, src/view/layers.rs:177 |
| `MIDI note must be 0–127` | 2 | 集める | src/view/drums.rs:197, src/view/drums.rs:199 |
| `Master effects editing requires SMF format 1` | 2 | 集める | src/player/resident.rs:147, src/player/selection.rs:115 |
| `Master: {e}` | 2 | 集める | src/play/audition_offline.rs:66, src/play/audition_output.rs:77 |
| `Missing container sound` | 2 | 集める | src/play/arrangement.rs:56, src/view/patterns/preview.rs:46 |
| `Missing edit scene` | 2 | 集める | src/score/pattern/editing.rs:48, src/score/pattern/editing.rs:76 |
| `Missing instrument layer` | 2 | 集める | src/play/audition_native.rs:31, src/play/audition_native.rs:44 |
| `Missing measure end` | 2 | 集める | src/view/macro_editor.rs:128, src/view/parameter_editor/preview.rs:78 |
| `Missing measure start` | 2 | 集める | src/view/macro_editor.rs:126, src/view/parameter_editor/preview.rs:76 |
| `Missing pattern meter` | 2 | 集める | src/view/mml_input/insertion.rs:20, src/view/patterns/navigation.rs:89 |
| `Missing scene pattern` | 2 | 集める | src/score/scene.rs:160, src/score/scene/projection.rs:23 |
| `Missing sound container` | 2 | 集める | src/view/patterns/library.rs:52, src/view/patterns/navigation.rs:131 |
| `Missing track` | 2 | 集める | src/score/same_measure.rs:138, src/score/same_measure/insert.rs:25 |
| `No audio output device` | 2 | 集める | src/play/audition_output.rs:35, src/play/wav.rs:89 |
| `No drum settings` | 2 | 集める | src/view/drums.rs:172, src/view/drums.rs:201 |
| `No notes` | 2 | 集める | src/view/recent/prefetch.rs:200, src/view/recent/prefetch.rs:201 |
| `Note crosses measure boundary` | 2 | 集める | src/score/same_measure.rs:108, src/score/same_measure.rs:119 |
| `Parameter control must be finite` | 2 | 集める | src/edit/parameter_curve/mod.rs:91, src/edit/parameter_curve/mod.rs:121 |
| `Parameter disappeared` | 2 | 集める | src/view/parameter_editor.rs:222, src/view/parameter_editor.rs:248 |
| `Parameter owner changed` | 2 | 集める | src/play/audition_parameters.rs:34, src/play/audition_parameters.rs:51 |
| `Parameter value is outside its native range` | 2 | 集める | src/edit/parameter_curve/mod.rs:88, src/edit/parameter_curve/mod.rs:140 |
| `Pattern edit target no longer exists` | 2 | 集める | src/score/pattern/editing.rs:20, src/score/pattern/editing.rs:32 |
| `Pattern range overflow` | 2 | 集める | src/score/pattern.rs:111, src/score/pattern/operations.rs:204 |
| `Pattern requires PPQN timing` | 2 | 集める | src/score/pattern/body.rs:45, src/score/pattern/operations.rs:70 |
| `Recording exceeds the pattern measure limit` | 2 | 集める | src/score/pattern/recording.rs:21, src/score/pattern/recording/capture.rs:199 |
| `Recording is empty` | 2 | 集める | src/score/pattern/recording/capture.rs:133, src/score/pattern/recording/recorder.rs:206 |
| `Recording requires PPQN timing` | 2 | 集める | src/score/pattern/recording/capture.rs:127, src/score/pattern/recording/recorder.rs:41 |
| `Recording requires a positive tempo` | 2 | 集める | src/score/pattern/recording/capture.rs:130, src/score/pattern/recording/recorder.rs:44 |
| `Reference Tempo requires format 1 track 0` | 2 | 集める | src/edit/reference_tempo.rs:38, src/edit/reference_tempo.rs:78 |
| `Select a container` | 2 | 集める | src/view/command_input/pattern_args.rs:169, src/view/patterns/membership.rs:14 |
| `Select a pattern or scene` | 2 | 集める | src/view/patterns/workspace.rs:243, src/view/patterns/workspace.rs:300 |
| `Select a scene` | 2 | 集める | src/view/patterns/workspace/commands.rs:222, src/view/patterns/workspace/dialog.rs:90 |
| `The duration must be at least 1 tick` | 2 | 集める | src/edit/note_length.rs:15, src/view/piano_roll/drag.rs:61 |
| `This patch has no routed macros` | 2 | 集める | src/view/macro_editor.rs:118, src/view/macro_editor.rs:202 |
| `Track was removed` | 2 | 集める | src/play/mixer/volume.rs:10, src/view/track_rename.rs:111 |
| `Trill interval is shorter than the available tick resolution` | 2 | 集める | src/edit/trill/rebuild/schedule.rs:73, src/edit/trill/rebuild/schedule.rs:82 |
| `Unsupported audio format: {format:?}` | 2 | 集める | src/play/audition_output.rs:226, src/play/wav.rs:96 |
| `VOICEVOX engine is not set` | 2 | 集める | src/view/voicevox.rs:51, src/view/voicevox/singer.rs:99 |

## その場で訳す（1か所）

| 文 | 使用箇所数 | 区分 | 使用箇所 |
|---|---:|---|---|
| `--player does not name an executable file` | 1 | その場で訳す | src/player/selection.rs:38 |
| `A control point must stay inside its segment` | 1 | その場で訳す | src/edit/tempo_curve.rs:70 |
| `A curve point already exists at this tick` | 1 | その場で訳す | src/edit/scalar_curve.rs:70 |
| `A dedicated parameter track needs one curve outside the conductor` | 1 | その場で訳す | src/edit/parameter_curve/metadata.rs:101 |
| `A new trill region needs both source pitches` | 1 | その場で訳す | src/edit/trill/rebuild.rs:106 |
| `A note crosses into a trill region` | 1 | その場で訳す | src/edit/trill/rebuild.rs:81 |
| `A parameter control must stay inside its segment` | 1 | その場で訳す | src/edit/parameter_curve/mod.rs:81 |
| `A parameter curve needs one control per segment` | 1 | その場で訳す | src/edit/parameter_curve/mod.rs:70 |
| `A parameter track cannot also own an instrument or notes` | 1 | その場で訳す | src/edit/parameter_curve/metadata.rs:113 |
| `A tempo curve BPM is out of the SMF range` | 1 | その場で訳す | src/edit/tempo_curve.rs:78 |
| `A tempo curve needs at least one tempo point` | 1 | その場で訳す | src/edit/tempo_curve.rs:60 |
| `A tempo curve needs one control point per segment` | 1 | その場で訳す | src/edit/tempo_curve.rs:63 |
| `A tempo curve requires one tick-zero metadata event in format 1 track 0` | 1 | その場で訳す | src/edit/tempo_curve.rs:104 |
| `A trill note has no Note Off` | 1 | その場で訳す | src/edit/trill/rebuild.rs:86 |
| `Add pattern takes no existing pattern arguments` | 1 | その場で訳す | src/view/command_input.rs:128 |
| `Ambiguous Macro {n}` | 1 | その場で訳す | src/view/macro_editor.rs:150 |
| `Ambiguous parameter ID: {}` | 1 | その場で訳す | src/play/audition_native.rs:185 |
| `An unmatched Note Off lies inside a trill region` | 1 | その場で訳す | src/edit/trill/rebuild.rs:68 |
| `Another command owns input` | 1 | その場で訳す | src/view/commands/execution.rs:66 |
| `Another editor owns input` | 1 | その場で訳す | src/view/commands/execution.rs:88 |
| `Arrangement blocks overlap on the same lane` | 1 | その場で訳す | src/score/scene/arrangement.rs:74 |
| `Arrangement overflow` | 1 | その場で訳す | src/score/scene/arrangement.rs:69 |
| `Arrangement requires PPQN timing` | 1 | その場で訳す | src/score/scene/arrangement.rs:60 |
| `Audio automation: {error}` | 1 | その場で訳す | src/play/audition_automation.rs:55 |
| `Audio baseline restoration: {error}` | 1 | その場で訳す | src/play/audition_automation.rs:16 |
| `Audio: plugin stopped while rendering` | 1 | その場で訳す | src/play/audition_offline.rs:217 |
| `Audition tempo has no pattern target` | 1 | その場で訳す | src/view/command_input.rs:139 |
| `Automation instrument changed` | 1 | その場で訳す | src/view/parameter_editor/curve.rs:14 |
| `Automation instrument changed: {}` | 1 | その場で訳す | src/play/audition_native.rs:175 |
| `Automation owner is outside playback scope` | 1 | その場で訳す | src/play/automation.rs:129 |
| `Automation owner was not opened` | 1 | その場で訳す | src/play/audition_output.rs:105 |
| `Automation parameter is unavailable` | 1 | その場で訳す | src/view/parameter_editor/curve.rs:19 |
| `Automation requires a format 1 instrument owner track` | 1 | その場で訳す | src/edit/parameter_curve/metadata.rs:124 |
| `BPM is outside the MIDI tempo range` | 1 | その場で訳す | src/edit/meta_input.rs:35 |
| `Backup sequence exhausted` | 1 | その場で訳す | src/store/backup.rs:38 |
| `Bypass change must keep the chain` | 1 | その場で訳す | src/play/audition_native.rs:79 |
| `Cache directory unavailable` | 1 | その場で訳す | src/view/recent/prefetch.rs:118 |
| `Cannot create plugin scanner` | 1 | その場で訳す | src/player/setup/scan.rs:25 |
| `Cannot delete the last SMF track` | 1 | その場で訳す | src/view/event_selection/actions.rs:89 |
| `Cannot kill VOICEVOX engine (PID {pid}): {}` | 1 | その場で訳す | src/voicevox/engine.rs:102 |
| `Cannot listen: {e}` | 1 | その場で訳す | src/player/resident_link.rs:300 |
| `Cannot open recording instrument` | 1 | その場で訳す | src/view/recording/phrase.rs:156 |
| `Cannot open {}: {e}` | 1 | その場で訳す | src/view/files/backup_tip.rs:123 |
| `Cannot prepare drum loop` | 1 | その場で訳す | src/view/drums/playback.rs:57 |
| `Cannot reach cat-plugin-player: {e}` | 1 | その場で訳す | src/player/resident_link.rs:377 |
| `Cannot rename config.json to status.json: {error}` | 1 | その場で訳す | src/store/config.rs:122 |
| `Cannot run netstat: {e}` | 1 | その場で訳す | src/voicevox/engine.rs:94 |
| `Cannot run taskkill: {e}` | 1 | その場で訳す | src/voicevox/engine.rs:99 |
| `Cannot save {}: {e}` | 1 | その場で訳す | src/store/saved.rs:207 |
| `Cannot start player: {e}` | 1 | その場で訳す | src/player/resident_link.rs:301 |
| `Cannot start plugin scan` | 1 | その場で訳す | src/player/setup/scan.rs:35 |
| `Cannot start {}: {e}` | 1 | その場で訳す | src/voicevox/engine.rs:55 |
| `Chained same measure reference` | 1 | その場で訳す | src/score/same_measure.rs:168 |
| `Channel must be 1–16` | 1 | その場で訳す | src/view/command_input/pattern_args.rs:14 |
| `Chord guide already exists` | 1 | その場で訳す | src/score/chord_guide.rs:57 |
| `Chord guide requires Format 1 PPQN` | 1 | その場で訳す | src/score/chord_guide.rs:54 |
| `Command target changed` | 1 | その場で訳す | src/view/commands/execution.rs:63 |
| `Conflicting parameter curves: {} / {}` | 1 | その場で訳す | src/score/automation.rs:45 |
| `Container instrument is incompatible with {}` | 1 | その場で訳す | src/view/parameter_editor/pattern_targets.rs:19 |
| `Container macro baseline domain changed` | 1 | その場で訳す | src/play/automation/pattern.rs:117 |
| `Container macro instrument changed` | 1 | その場で訳す | src/play/automation/pattern.rs:109 |
| `Could not answer the player: {error}` | 1 | その場で訳す | src/player/resident_link.rs:195 |
| `Could not read the plugin state: {error}` | 1 | その場で訳す | src/player/resident.rs:34 |
| `Create a scene first` | 1 | その場で訳す | src/view/patterns/workspace.rs:198 |
| `Create at least two Macro variations first` | 1 | その場で訳す | src/view/parameter_editor/variation.rs:53 |
| `Curve points must remain in tick order` | 1 | その場で訳す | src/edit/scalar_curve.rs:109 |
| `Delta time is too large` | 1 | その場で訳す | src/edit/mod.rs:61 |
| `Desktop directory is unavailable` | 1 | その場で訳す | src/player/setup/download.rs:19 |
| `Destination contains notes` | 1 | その場で訳す | src/score/same_measure.rs:174 |
| `Destination is occupied` | 1 | その場で訳す | src/score/scene/arrangement.rs:276 |
| `Destination parameter is unavailable` | 1 | その場で訳す | src/view/parameter_editor/pattern_targets.rs:28 |
| `Download worker stopped` | 1 | その場で訳す | src/player/setup.rs:82 |
| `Drum editing requires PPQN >= 4` | 1 | その場で訳す | src/score/drums.rs:124 |
| `Drum measure overflow` | 1 | その場で訳す | src/score/drums/measures.rs:41 |
| `Drum note has no Note Off` | 1 | その場で訳す | src/score/drums/measures.rs:159 |
| `Drum range is outside the pattern length` | 1 | その場で訳す | src/view/drums/playback.rs:31 |
| `Drum sequencer requires its four instrument rows` | 1 | その場で訳す | src/score/drums.rs:74 |
| `Drum step exceeds tick range` | 1 | その場で訳す | src/score/drums.rs:128 |
| `Duplicate Reference Tempo tick` | 1 | その場で訳す | src/edit/reference_tempo.rs:65 |
| `Duplicate identity across document roles` | 1 | その場で訳す | src/score/pattern/catalog.rs:95 |
| `Duplicate or misplaced scene metadata` | 1 | その場で訳す | src/score/scene/persistence.rs:9 |
| `Duplicate or misplaced track identity` | 1 | その場で訳す | src/score/pattern/catalog.rs:20 |
| `Duplicate or zero arrangement identity` | 1 | その場で訳す | src/score/scene/persistence.rs:40 |
| `Duplicate, misplaced or oversized drum metadata` | 1 | その場で訳す | src/score/drums.rs:86 |
| `Edit exceeds the selected pattern body` | 1 | その場で訳す | src/score/pattern/editing.rs:125 |
| `Edit range overflow` | 1 | その場で訳す | src/score/pattern/editing.rs:61 |
| `Empty cell` | 1 | その場で訳す | src/score/scene/arrangement.rs:387 |
| `Empty source cell` | 1 | その場で訳す | src/score/scene/arrangement.rs:274 |
| `Enter a name, positive length and lane count` | 1 | その場で訳す | src/score/scene/arrangement.rs:165 |
| `Enter a positive measure number` | 1 | その場で訳す | src/view/command_input/same_measure.rs:46 |
| `Enter a positive scene length` | 1 | その場で訳す | src/view/patterns/workspace/dialog.rs:80 |
| `Enter meter fractions separated by spaces` | 1 | その場で訳す | src/score/pattern/body.rs:109 |
| `Expected a single-line chord` | 1 | その場で訳す | src/score/chord_guide.rs:37 |
| `Expected channel, CC number or bend, value` | 1 | その場で訳す | src/view/command_input/pattern_args.rs:8 |
| `Expected major or minor` | 1 | その場で訳す | src/edit/meta_input.rs:50 |
| `Generated state was not retained` | 1 | その場で訳す | src/score/state_cache.rs:153 |
| `Initial meta events require format 1 track 0` | 1 | その場で訳す | src/edit/initial_meta.rs:14 |
| `Instance group IDs exhausted` | 1 | その場で訳す | src/edit/parameter_curve/metadata.rs:140 |
| `Instrument changed for parameter {}` | 1 | その場で訳す | src/score/automation.rs:69 |
| `Invalid BPM` | 1 | その場で訳す | src/edit/meta_input.rs:32 |
| `Invalid MBT` | 1 | その場で訳す | src/view/note_edit.rs:127 |
| `Invalid MIDI baseline value` | 1 | その場で訳す | src/view/command_input/pattern_args.rs:31 |
| `Invalid MIDI channel` | 1 | その場で訳す | src/edit/mml.rs:181 |
| `Invalid Reference Tempo metadata` | 1 | その場で訳す | src/edit/reference_tempo.rs:43 |
| `Invalid Reference Tempo rank` | 1 | その場で訳す | src/edit/reference_tempo.rs:53 |
| `Invalid Song lane` | 1 | その場で訳す | src/score/scene/arrangement.rs:53 |
| `Invalid Song placement` | 1 | その場で訳す | src/score/scene/persistence.rs:59 |
| `Invalid Trill End metadata` | 1 | その場で訳す | src/edit/trill.rs:99 |
| `Invalid Trill Start metadata` | 1 | その場で訳す | src/edit/trill.rs:91 |
| `Invalid arrangement lane` | 1 | その場で訳す | src/score/scene/arrangement.rs:47 |
| `Invalid audition tempo` | 1 | その場で訳す | src/view/command_input/pattern_args.rs:138 |
| `Invalid audition tempo or PPQN timing` | 1 | その場で訳す | src/score/audition_timing.rs:11 |
| `Invalid body range` | 1 | その場で訳す | src/score/pattern/body.rs:64 |
| `Invalid container baselines` | 1 | その場で訳す | src/score/pattern/automation.rs:51 |
| `Invalid create input` | 1 | その場で訳す | src/view/command_input/pattern_args.rs:176 |
| `Invalid curve point` | 1 | その場で訳す | src/edit/scalar_curve.rs:100 |
| `Invalid dB` | 1 | その場で訳す | src/view/layers.rs:180 |
| `Invalid denominator` | 1 | その場で訳す | src/score/pattern/body.rs:119 |
| `Invalid detected trill interval` | 1 | その場で訳す | src/edit/trill.rs:170 |
| `Invalid drum sequencer settings` | 1 | その場で訳す | src/score/drums.rs:52 |
| `Invalid drum step` | 1 | その場で訳す | src/score/drums/measures.rs:89 |
| `Invalid duration` | 1 | その場で訳す | src/view/note_edit.rs:136 |
| `Invalid event selection` | 1 | その場で訳す | src/edit/event_range.rs:7 |
| `Invalid generated state size` | 1 | その場で訳す | src/score/state_cache.rs:166 |
| `Invalid integer octave` | 1 | その場で訳す | src/view/layers.rs:179 |
| `Invalid integer tick` | 1 | その場で訳す | src/view/parameter_editor/curve.rs:316 |
| `Invalid key signature (e.g. D minor)` | 1 | その場で訳す | src/edit/meta_input.rs:43 |
| `Invalid macro values` | 1 | その場で訳す | src/score/macro_values.rs:30 |
| `Invalid macro variation curve` | 1 | その場で訳す | src/score/macro_curve.rs:90 |
| `Invalid macro variations` | 1 | その場で訳す | src/score/macro_values.rs:42 |
| `Invalid measure count` | 1 | その場で訳す | src/score/pattern/body.rs:131 |
| `Invalid measure end` | 1 | その場で訳す | src/score/same_measure.rs:62 |
| `Invalid measure range` | 1 | その場で訳す | src/score/same_measure/insert.rs:15 |
| `Invalid measure start` | 1 | その場で訳す | src/score/same_measure.rs:57 |
| `Invalid note` | 1 | その場で訳す | src/edit/note_value.rs:18 |
| `Invalid note name` | 1 | その場で訳す | src/edit/note_value.rs:14 |
| `Invalid numerator` | 1 | その場で訳す | src/score/pattern/body.rs:114 |
| `Invalid or duplicate MIDI baseline` | 1 | その場で訳す | src/score/pattern/automation.rs:72 |
| `Invalid or duplicate fixed macro target` | 1 | その場で訳す | src/score/macro_values.rs:82 |
| `Invalid or duplicate instance group identity` | 1 | その場で訳す | src/edit/parameter_curve/metadata.rs:93 |
| `Invalid or duplicate parameter baseline` | 1 | その場で訳す | src/score/pattern/automation.rs:66 |
| `Invalid or duplicate scene cell` | 1 | その場で訳す | src/score/scene.rs:154 |
| `Invalid or duplicate track volume metadata` | 1 | その場で訳す | src/play/track_volume.rs:26 |
| `Invalid parameter curve version or target identity` | 1 | その場で訳す | src/edit/parameter_curve/mod.rs:61 |
| `Invalid pattern curve ownership or range` | 1 | その場で訳す | src/score/pattern/automation.rs:101 |
| `Invalid pattern identity or container` | 1 | その場で訳す | src/score/pattern/catalog.rs:42 |
| `Invalid pattern meter` | 1 | その場で訳す | src/score/pattern/body.rs:28 |
| `Invalid pattern version or identity` | 1 | その場で訳す | src/score/pattern.rs:68 |
| `Invalid performance owner` | 1 | その場で訳す | src/score/performance.rs:64 |
| `Invalid performance parameter window` | 1 | その場で訳す | src/score/performance.rs:83 |
| `Invalid performance parameter: {e}` | 1 | その場で訳す | src/score/performance.rs:68 |
| `Invalid same measure payload` | 1 | その場で訳す | src/score/same_measure.rs:26 |
| `Invalid scene identity or length` | 1 | その場で訳す | src/score/scene.rs:124 |
| `Invalid scene metadata: {e}` | 1 | その場で訳す | src/score/scene/persistence.rs:13 |
| `Invalid scene meter` | 1 | その場で訳す | src/score/scene.rs:94 |
| `Invalid scene version` | 1 | その場で訳す | src/score/scene/persistence.rs:19 |
| `Invalid tempo event` | 1 | その場で訳す | src/edit/reference_tempo.rs:84 |
| `Invalid tick` | 1 | その場で訳す | src/view/command_input/guide_edit.rs:67 |
| `Invalid ticks per quarter note` | 1 | その場で訳す | src/edit/tracks.rs:127 |
| `Invalid trill tempo event` | 1 | その場で訳す | src/edit/trill/rebuild/schedule.rs:26 |
| `Invalid variation instrument identity` | 1 | その場で訳す | src/score/macro_curve.rs:28 |
| `Invalid velocity` | 1 | その場で訳す | src/view/note_edit.rs:115 |
| `Invalid zero tempo` | 1 | その場で訳す | src/edit/reference_tempo.rs:88 |
| `Invalid zero trill tempo` | 1 | その場で訳す | src/edit/trill/rebuild/schedule.rs:30 |
| `Key signature must use at most seven sharps or flats` | 1 | その場で訳す | src/edit/meta_input.rs:66 |
| `LOCALAPPDATA is unavailable` | 1 | その場で訳す | src/app/startup.rs:47 |
| `Lane requires exactly one owner` | 1 | その場で訳す | src/score/scene/persistence.rs:47 |
| `Layer ID exhausted` | 1 | その場で訳す | src/view/layers.rs:95 |
| `Layer selection target no longer exists or is active` | 1 | その場で訳す | src/player/resident.rs:98 |
| `Loop recording requires at least one measure` | 1 | その場で訳す | src/score/pattern/recording/recorder.rs:47 |
| `MML exceeds the pattern measure limit` | 1 | その場で訳す | src/view/mml_input/insertion.rs:23 |
| `MML insertion requires ticks per quarter note` | 1 | その場で訳す | src/edit/mml.rs:131 |
| `MML starts before the pattern` | 1 | その場で訳す | src/view/mml_input/insertion.rs:15 |
| `Macro values are too large` | 1 | その場で訳す | src/score/macro_values.rs:113 |
| `Macro variation curve instrument changed` | 1 | その場で訳す | src/score/macro_curve.rs:176 |
| `Macro variation curve references a missing variation` | 1 | その場で訳す | src/score/macro_curve.rs:168 |
| `Macro variation differs from active values` | 1 | その場で訳す | src/score/macro_values.rs:51 |
| `Macro variation instrument changed` | 1 | その場で訳す | src/view/parameter_editor/variation.rs:57 |
| `Macro variation targets differ` | 1 | その場で訳す | src/score/macro_values.rs:61 |
| `Malformed pattern MIDI control` | 1 | その場で訳す | src/score/pattern/render.rs:36 |
| `Malformed pattern note` | 1 | その場で訳す | src/score/pattern/body.rs:70 |
| `Managed trill notes use the Trill interval` | 1 | その場で訳す | src/edit/trill.rs:365 |
| `Measure exceeds tick range` | 1 | その場で訳す | src/score/drums/measures.rs:23 |
| `Measure is outside the pattern length` | 1 | その場で訳す | src/view/drums/playback.rs:14 |
| `Measure lengths differ` | 1 | その場で訳す | src/score/same_measure.rs:165 |
| `Measure must be positive` | 1 | その場で訳す | src/score/drums/measures.rs:14 |
| `Measure number must be positive` | 1 | その場で訳す | src/score/same_measure.rs:49 |
| `Meter cannot be represented by whole ticks` | 1 | その場で訳す | src/score/pattern/body.rs:122 |
| `Misplaced or duplicate macro values` | 1 | その場で訳す | src/score/macro_values.rs:97 |
| `Misplaced or duplicate macro variation curves` | 1 | その場で訳す | src/score/macro_curve.rs:50 |
| `Missing Macro {n} parameter` | 1 | その場で訳す | src/view/macro_editor.rs:148 |
| `Missing Macro {n} saved position` | 1 | その場で訳す | src/view/macro_editor.rs:187 |
| `Missing Scene` | 1 | その場で訳す | src/score/scene/sizing.rs:8 |
| `Missing Trill Start` | 1 | その場で訳す | src/edit/trill.rs:229 |
| `Missing arrangement pattern` | 1 | その場で訳す | src/score/scene/arrangement.rs:102 |
| `Missing automation layer: {}` | 1 | その場で訳す | src/play/audition_native.rs:169 |
| `Missing automation owner` | 1 | その場で訳す | src/view/parameter_editor/persistence.rs:56 |
| `Missing conductor` | 1 | その場で訳す | src/score/performance.rs:141 |
| `Missing conductor track` | 1 | その場で訳す | src/score/scene/persistence.rs:72 |
| `Missing container baseline for channel {} {}` | 1 | その場で訳す | src/score/pattern/render.rs:151 |
| `Missing container macro layer` | 1 | その場で訳す | src/play/automation/pattern.rs:102 |
| `Missing container parameter baseline` | 1 | その場で訳す | src/score/pattern/render.rs:102 |
| `Missing container patch` | 1 | その場で訳す | src/view/parameter_editor/pattern_targets.rs:78 |
| `Missing created pattern` | 1 | その場で訳す | src/view/patterns/library.rs:63 |
| `Missing destination container` | 1 | その場で訳す | src/score/pattern/automation.rs:228 |
| `Missing destination sound` | 1 | その場で訳す | src/view/patterns/membership.rs:18 |
| `Missing drum container` | 1 | その場で訳す | src/view/drums/pattern.rs:33 |
| `Missing drum measure map` | 1 | その場で訳す | src/score/drums/measures.rs:141 |
| `Missing drum track` | 1 | その場で訳す | src/score/drums/measures.rs:92 |
| `Missing edit track` | 1 | その場で訳す | src/score/pattern/editing.rs:114 |
| `Missing fixed macro baseline` | 1 | その場で訳す | src/play/automation/pattern.rs:112 |
| `Missing layer` | 1 | その場で訳す | src/view/parameter_editor.rs:115 |
| `Missing layer for parameter {}` | 1 | その場で訳す | src/score/automation.rs:60 |
| `Missing macro owner` | 1 | その場で訳す | src/view/parameter_editor/variation.rs:45 |
| `Missing macro variations for curve` | 1 | その場で訳す | src/score/macro_curve.rs:163 |
| `Missing measure meter` | 1 | その場で訳す | src/score/drums/measures.rs:32 |
| `Missing meter` | 1 | その場で訳す | src/score/scene/arrangement.rs:169 |
| `Missing or incompatible container parameter baseline` | 1 | その場で訳す | src/score/pattern/catalog.rs:59 |
| `Missing or invalid scene lane` | 1 | その場で訳す | src/score/scene.rs:129 |
| `Missing owner for parameter {}` | 1 | その場で訳す | src/score/automation.rs:51 |
| `Missing owner: {} / {}` | 1 | その場で訳す | src/play/automation.rs:119 |
| `Missing parameter baseline` | 1 | その場で訳す | src/score/pattern/automation.rs:122 |
| `Missing parameter: {} / {}` | 1 | その場で訳す | src/play/audition_native.rs:183 |
| `Missing pattern edit base` | 1 | その場で訳す | src/view/mml_input/insertion.rs:12 |
| `Missing pattern occurrence` | 1 | その場で訳す | src/score/pattern/render.rs:83 |
| `Missing pattern parameter` | 1 | その場で訳す | src/view/parameter_editor/persistence.rs:13 |
| `Missing placement lane or measure` | 1 | その場で訳す | src/score/scene/arrangement.rs:64 |
| `Missing preview pattern` | 1 | その場で訳す | src/view/patterns/preview.rs:11 |
| `Missing recorded pattern` | 1 | その場で訳す | src/view/recording/take.rs:22 |
| `Missing scene cell` | 1 | その場で訳す | src/score/scene/arrangement.rs:226 |
| `Missing song meter` | 1 | その場で訳す | src/score/scene/arrangement/occurrences.rs:16 |
| `Missing sound owner` | 1 | その場で訳す | src/player/selection.rs:195 |
| `Missing trill pattern` | 1 | その場で訳す | src/edit/trill.rs:303 |
| `Missing {prefix} saved position` | 1 | その場で訳す | src/view/macro_editor/preview.rs:75 |
| `Multiple references in destination` | 1 | その場で訳す | src/score/same_measure.rs:171 |
| `Multiple tracks share this parameter target` | 1 | その場で訳す | src/edit/parameter_curve/metadata.rs:150 |
| `No MIDI document is loaded` | 1 | その場で訳す | src/store/saved.rs:191 |
| `No Reference Tempo at this tick` | 1 | その場で訳す | src/edit/reference_tempo.rs:123 |
| `No SMF track is loaded` | 1 | その場で訳す | src/store/auto_save.rs:31 |
| `No continuous automatable parameters` | 1 | その場で訳す | src/view/parameter_editor.rs:170 |
| `No executable directory` | 1 | その場で訳す | src/player/setup.rs:46 |
| `No guide event selected` | 1 | その場で訳す | src/view/command_input.rs:187 |
| `No input` | 1 | その場で訳す | src/view/drums.rs:195 |
| `No installed {} patches in the catalog` | 1 | その場で訳す | src/view/drums/catalog.rs:92 |
| `No instrument` | 1 | その場で訳す | src/view/macro_editor/preview.rs:28 |
| `No local application directory` | 1 | その場で訳す | src/view/drums/catalog.rs:37 |
| `No matching parameter` | 1 | その場で訳す | src/view/parameter_editor.rs:221 |
| `No measure selection` | 1 | その場で訳す | src/view/command_input/same_measure.rs:27 |
| `No numeric field selected` | 1 | その場で訳す | src/view/layers.rs:181 |
| `No parameter selection` | 1 | その場で訳す | src/view/parameter_editor.rs:217 |
| `No routed macros` | 1 | その場で訳す | src/view/macro_editor.rs:110 |
| `No selected layer` | 1 | その場で訳す | src/player/bulk.rs:70 |
| `No selected point` | 1 | その場で訳す | src/view/parameter_editor/curve.rs:318 |
| `No such curve point` | 1 | その場で訳す | src/edit/scalar_curve.rs:84 |
| `No such effect slot` | 1 | その場で訳す | src/player/resident.rs:177 |
| `No such parameter control` | 1 | その場で訳す | src/edit/parameter_curve/mod.rs:127 |
| `No such parameter segment` | 1 | その場で訳す | src/edit/parameter_curve/mod.rs:124 |
| `No value input` | 1 | その場で訳す | src/view/parameter_editor/curve.rs:315 |
| `No value is being edited` | 1 | その場で訳す | src/view/layers.rs:171 |
| `Note crosses pattern boundary` | 1 | その場で訳す | src/score/pattern/body.rs:82 |
| `Note edit target changed` | 1 | その場で訳す | src/view/note_edit.rs:102 |
| `Note has no matching end` | 1 | その場で訳す | src/edit/note_value.rs:38 |
| `Overlapping pattern control {control:?}` | 1 | その場で訳す | src/score/pattern/render.rs:135 |
| `Overlapping performance parameter windows` | 1 | その場で訳す | src/score/performance.rs:95 |
| `Owner track {} is unavailable` | 1 | その場で訳す | src/score/automation.rs:55 |
| `Parameter automation requires SMF format 1` | 1 | その場で訳す | src/view/parameter_editor.rs:152 |
| `Parameter format/value space mismatch` | 1 | その場で訳す | src/edit/parameter_curve/mod.rs:66 |
| `Parameter instrument is unavailable` | 1 | その場で訳す | src/score/automation.rs:67 |
| `Parameter is missing or not automatable: {} / {}` | 1 | その場で訳す | src/score/automation.rs:86 |
| `Parameter metadata is too large` | 1 | その場で訳す | src/edit/parameter_curve/metadata.rs:69 |
| `Parameter metadata requires tick zero in SMF format 1` | 1 | その場で訳す | src/edit/parameter_curve/metadata.rs:83 |
| `Parameter points must have unique increasing ticks` | 1 | その場で訳す | src/edit/parameter_curve/mod.rs:73 |
| `Parameter range or value space changed: {} / {}` | 1 | その場で訳す | src/score/automation.rs:95 |
| `Parameter target changed during editing` | 1 | その場で訳す | src/edit/parameter_curve/metadata.rs:194 |
| `Patch or plugin changed during state generation` | 1 | その場で訳す | src/score/state_cache.rs:145 |
| `Patch state requires session restoration` | 1 | その場で訳す | src/score/instrument.rs:8 |
| `Pattern and Song meters differ` | 1 | その場で訳す | src/score/scene/arrangement.rs:108 |
| `Pattern contains an unmatched or carried note off` | 1 | その場で訳す | src/score/pattern/body.rs:88 |
| `Pattern contains an unterminated note` | 1 | その場で訳す | src/score/pattern/body.rs:99 |
| `Pattern contains same measure reference` | 1 | その場で訳す | src/score/pattern/catalog.rs:66 |
| `Pattern content extends beyond the requested length` | 1 | その場で訳す | src/score/pattern/operations.rs:201 |
| `Pattern curve time overflow` | 1 | その場で訳す | src/play/automation/pattern.rs:151 |
| `Pattern curves must be stored at the body start` | 1 | その場で訳す | src/score/pattern/automation.rs:89 |
| `Pattern drum-row interpretation is incompatible with the destination container` | 1 | その場で訳す | src/score/pattern/operations.rs:157 |
| `Pattern edit has no placement` | 1 | その場で訳す | src/score/pattern/editing.rs:42 |
| `Pattern edit target changed` | 1 | その場で訳す | src/score/pattern/editing.rs:37 |
| `Pattern event overflow` | 1 | その場で訳す | src/score/pattern/render.rs:129 |
| `Pattern input target changed` | 1 | その場で訳す | src/view/command_input/pattern_args.rs:131 |
| `Pattern is in use` | 1 | その場で訳す | src/score/pattern/operations.rs:128 |
| `Pattern length overflow` | 1 | その場で訳す | src/score/pattern/body.rs:19 |
| `Pattern meter does not match its exact length` | 1 | その場で訳す | src/score/pattern/body.rs:49 |
| `Pattern occurrence exceeds the declared length` | 1 | その場で訳す | src/score/pattern/render.rs:88 |
| `Pattern occurrence overflow` | 1 | その場で訳す | src/score/pattern/render.rs:86 |
| `Pattern requires an explicit length` | 1 | その場で訳す | src/score/pattern/body.rs:15 |
| `Pattern storage ranges overlap or have no boundary gap` | 1 | その場で訳す | src/score/pattern/catalog.rs:74 |
| `Performance export must use a different file from the editable MID` | 1 | その場で訳す | src/view/files/performance.rs:39 |
| `Performance expression windows cannot share an owner with ordinary parameter tracks` | 1 | その場で訳す | src/play/automation/performance.rs:20 |
| `Pitch must be 0–127` | 1 | その場で訳す | src/edit/note_value.rs:10 |
| `Placement exceeds the declared Song length` | 1 | その場で訳す | src/score/scene/arrangement.rs:78 |
| `Player restoration failed: {}` | 1 | その場で訳す | src/player/state_request.rs:43 |
| `Player returned an invalid state size` | 1 | その場で訳す | src/player/state_request.rs:48 |
| `Player root discovery failed: {}` | 1 | その場で訳す | src/player/patch_roots.rs:60 |
| `Player stdin unavailable` | 1 | その場で訳す | src/player/state_request.rs:37 |
| `Plugin state has no restoration reference` | 1 | その場で訳す | src/score/layer_track.rs:49 |
| `Prefetch worker stopped` | 1 | その場で訳す | src/view/recent/prefetch.rs:301 |
| `Program Change cannot change a pattern container's sound` | 1 | その場で訳す | src/score/pattern/render.rs:33 |
| `Published cat-plugin-player is too old for this editor (requires --resident, --restore-state and --patch-roots). A newer player release is required.` | 1 | その場で訳す | src/player/setup/download.rs:110 |
| `Recording container disappeared` | 1 | その場で訳す | src/view/recording/take.rs:12 |
| `Recording division changed` | 1 | その場で訳す | src/score/pattern/recording/recorder.rs:202 |
| `Recording has held notes` | 1 | その場で訳す | src/score/pattern/recording/recorder.rs:199 |
| `Recording has not started` | 1 | その場で訳す | src/view/recording/take.rs:5 |
| `Recording storage overflow` | 1 | その場で訳す | src/score/pattern/recording.rs:50 |
| `Reference must be at measure start` | 1 | その場で訳す | src/score/same_measure.rs:159 |
| `Render stopped` | 1 | その場で訳す | src/view/recent/prefetch.rs:318 |
| `SMF directory is missing: {}` | 1 | その場で訳す | src/score/catalog.rs:11 |
| `SMF events are out of tick order` | 1 | その場で訳す | src/edit/mod.rs:60 |
| `SMF filename is missing` | 1 | その場で訳す | src/store/backup.rs:16 |
| `Same measure number must be positive` | 1 | その場で訳す | src/score/same_measure.rs:29 |
| `Scene and song measure lengths differ` | 1 | その場で訳す | src/score/scene/arrangement.rs:93 |
| `Scene edit placement changed` | 1 | その場で訳す | src/score/pattern/editing.rs:56 |
| `Scene is in use` | 1 | その場で訳す | src/score/scene/arrangement.rs:315 |
| `Scene measure is outside its length` | 1 | その場で訳す | src/score/scene.rs:77 |
| `Scene meter cannot represent its exact measure boundaries` | 1 | その場で訳す | src/score/scene.rs:137 |
| `Scene pattern measure lengths differ` | 1 | その場で訳す | src/score/scene.rs:178 |
| `Scene placement outside its lanes` | 1 | その場で訳す | src/score/scene/persistence.rs:52 |
| `Scene placement overflow` | 1 | その場で訳す | src/score/scene.rs:166 |
| `Scene requires PPQN timing` | 1 | その場で訳す | src/score/scene.rs:134 |
| `Scenes contain patterns only` | 1 | その場で訳す | src/score/scene/arrangement.rs:238 |
| `Select a Scene` | 1 | その場で訳す | src/view/patterns/workspace/dialog.rs:69 |
| `Select a drum sound container` | 1 | その場で訳す | src/view/drums/pattern.rs:26 |
| `Select a guide event` | 1 | その場で訳す | src/view/command_input/guide_edit.rs:99 |
| `Select a key or chord` | 1 | その場で訳す | src/view/command_input/guide_edit.rs:105 |
| `Select a normal track` | 1 | その場で訳す | src/score/same_measure/insert.rs:12 |
| `Select a note track` | 1 | その場で訳す | src/view/mml_input.rs:49 |
| `Select a pattern` | 1 | その場で訳す | src/view/command_input/pattern_args.rs:174 |
| `Select a sound container` | 1 | その場で訳す | src/score/pattern/operations.rs:50 |
| `Select an instrument owner track` | 1 | その場で訳す | src/view/parameter_editor.rs:155 |
| `Select an instrument track` | 1 | その場で訳す | src/view/macro_editor.rs:102 |
| `Selection target is no longer loaded` | 1 | その場で訳す | src/player/bulk.rs:58 |
| `Self reference` | 1 | その場で訳す | src/score/same_measure.rs:162 |
| `ShellExecuteW failed ({result})` | 1 | その場で訳す | src/view/files/backup_tip.rs:119 |
| `Song changed` | 1 | その場で訳す | src/player/resident.rs:120 |
| `Song has no explicit performance length` | 1 | その場で訳す | src/score/performance.rs:137 |
| `Source and destination overlap` | 1 | その場で訳す | src/score/same_measure/insert.rs:20 |
| `State cache {}: {e}` | 1 | その場で訳す | src/score/state_cache.rs:218 |
| `State/reference plugin mismatch` | 1 | その場で訳す | src/score/layer_track.rs:52 |
| `Symlinked plugin bundles cannot be cached safely` | 1 | その場で訳す | src/score/state_cache.rs:75 |
| `Tempo points must be in increasing tick order` | 1 | その場で訳す | src/edit/tempo_curve.rs:66 |
| `The MIDI document changed` | 1 | その場で訳す | src/store/saved.rs:160 |
| `The duration is too large` | 1 | その場で訳す | src/edit/note_length.rs:21 |
| `The move crosses another note of the same key` | 1 | その場で訳す | src/edit/relocate.rs:50 |
| `The note cannot move before tick 0 or that far` | 1 | その場で訳す | src/edit/note_move.rs:18 |
| `The phrase's Capture pattern no longer exists` | 1 | その場で訳す | src/score/pattern/recording/capture.rs:320 |
| `The phrase's container no longer exists` | 1 | その場で訳す | src/view/recording/capture.rs:68 |
| `The resident player requires Windows` | 1 | その場で訳す | src/player/selection.rs:136 |
| `The start cannot move before tick 0` | 1 | その場で訳す | src/view/piano_roll/drag.rs:54 |
| `The start must be before the Note Off` | 1 | その場で訳す | src/edit/note_start.rs:15 |
| `This pattern action has no container input` | 1 | その場で訳す | src/view/command_input/pattern_args.rs:114 |
| `This pattern action has no value input` | 1 | その場で訳す | src/view/command_input/pattern_args.rs:111 |
| `Too many automated parameters on one layer` | 1 | その場で訳す | src/play/audition_native.rs:209 |
| `Too many instrument layers` | 1 | その場で訳す | src/view/layers.rs:80 |
| `Too many macro variation sets` | 1 | その場で訳す | src/score/macro_values.rs:33 |
| `Track has no instrument layer` | 1 | その場で訳す | src/player/resident_link.rs:99 |
| `Track has no layers` | 1 | その場で訳す | src/player/resident.rs:134 |
| `Track {}, tick {}: {error}` | 1 | その場で訳す | src/score/same_measure.rs:185 |
| `Trill detection requires format 1 with PPQN timing` | 1 | その場で訳す | src/edit/trill.rs:122 |
| `Trill durations must be finite and positive` | 1 | その場で訳す | src/edit/trill.rs:319 |
| `Trill end tick overflow` | 1 | その場で訳す | src/edit/trill.rs:167 |
| `Trill interval must be finite and positive` | 1 | その場で訳す | src/edit/trill.rs:338 |
| `Trill metadata requires format 1 with PPQN timing` | 1 | その場で訳す | src/edit/trill.rs:69 |
| `Trill notes overlap or cross their region boundary` | 1 | その場で訳す | src/edit/trill/rebuild.rs:89 |
| `Trill regeneration exceeds one million notes` | 1 | その場で訳す | src/edit/trill/rebuild/schedule.rs:109 |
| `Trill regions overlap on the same channel and pitch` | 1 | その場で訳す | src/edit/trill/rebuild.rs:32 |
| `Trill tempo curve requires tempos only in the conductor` | 1 | その場で訳す | src/edit/trill/rebuild/schedule.rs:20 |
| `Unknown event` | 1 | その場で訳す | src/edit/trill.rs:251 |
| `Unknown note` | 1 | その場で訳す | src/edit/trill.rs:355 |
| `Unmatched Trill End` | 1 | その場で訳す | src/edit/trill.rs:97 |
| `Unmatched Trill Start` | 1 | その場で訳す | src/edit/trill.rs:110 |
| `Unmatched trill metadata` | 1 | その場で訳す | src/edit/trill.rs:268 |
| `Unrepresentable audition tempo` | 1 | その場で訳す | src/score/audition_timing.rs:15 |
| `Unsupported CC baseline` | 1 | その場で訳す | src/view/command_input/pattern_args.rs:24 |
| `Unsupported measure length` | 1 | その場で訳す | src/score/drums/measures.rs:35 |
| `Unsupported patch catalog version` | 1 | その場で訳す | src/view/drums/catalog.rs:103 |
| `Unsupported tempo curve schema` | 1 | その場で訳す | src/edit/tempo_curve.rs:57 |
| `VOICEVOX /sing_frame_audio_query: not an object` | 1 | その場で訳す | src/voicevox/client.rs:68 |
| `VOICEVOX WAV: {actual} Hz, expected {rate} Hz` | 1 | その場で訳す | src/voicevox/cache.rs:49 |
| `VOICEVOX cannot automate plugin parameters: {}` | 1 | その場で訳す | src/play/automation.rs:122 |
| `VOICEVOX voice unavailable: {}` | 1 | その場で訳す | src/voicevox/client.rs:58 |
| `VOICEVOX: catalog thread panicked` | 1 | その場で訳す | src/view/voicevox/singer.rs:107 |
| `VOICEVOX: no cache directory` | 1 | その場で訳す | src/play/sung/prerender.rs:181 |
| `VOICEVOX: no humming voices` | 1 | その場で訳す | src/view/voicevox/singer.rs:90 |
| `VOICEVOX: prerender thread panicked` | 1 | その場で訳す | src/play/sung/prerender.rs:350 |
| `VOICEVOX: sample thread panicked` | 1 | その場で訳す | src/view/voicevox/singer/preview.rs:223 |
| `Velocity must be 1–127` | 1 | その場で訳す | src/edit/note_value.rs:45 |
| `WAV path has no directory` | 1 | その場で訳す | src/play/wav.rs:9 |
| `cat-plugin-player stopped: {error}` | 1 | その場で訳す | src/player/resident_link.rs:216 |
| `data before fmt` | 1 | その場で訳す | src/voicevox/pcm.rs:35 |
| `download exceeds size limit` | 1 | その場で訳す | src/player/setup/download.rs:44 |
| `expected 16-bit PCM stereo` | 1 | その場で訳す | src/voicevox/pcm.rs:30 |
| `invalid ({e})` | 1 | その場で訳す | src/score/macro_curve.rs:119 |
| `invalid release entry` | 1 | その場で訳す | src/player/setup/download.rs:121 |
| `no data chunk` | 1 | その場で訳す | src/voicevox/pcm.rs:51 |
| `not RIFF/WAVE` | 1 | その場で訳す | src/voicevox/pcm.rs:7 |
| `release is not a Windows executable` | 1 | その場で訳す | src/player/setup/download.rs:132 |
| `truncated data` | 1 | その場で訳す | src/voicevox/pcm.rs:38 |
| `{} did not answer on port {port} in {} s` | 1 | その場で訳す | src/voicevox/engine.rs:70 |
| `{} exited before answering ({status})` | 1 | その場で訳す | src/voicevox/engine.rs:64 |
| `{} is already connected in another slot` | 1 | その場で訳す | src/player/resident.rs:170 |

## その場で訳した文の訳

上の「その場で訳す」表の各文を、使う場所で次の訳に書き換えた。`invalid ({e})` は `message::invalid_value` に置き換えた。

| 原文 | 訳 |
|---|---|
| `--player does not name an executable file` | `--playerに指定したものが実行ファイルではありません` |
| `A control point must stay inside its segment` | `control pointは区間の中に置きます` |
| `A curve point already exists at this tick` | `このtickには既にcurveの点があります` |
| `A dedicated parameter track needs one curve outside the conductor` | `専用のparameter trackにはconductorの外にcurveが1本必要です` |
| `A new trill region needs both source pitches` | `新しいtrillの範囲には元の音高が2つとも必要です` |
| `A note crosses into a trill region` | `noteがtrillの範囲に入り込んでいます` |
| `A parameter control must stay inside its segment` | `parameterのcontrolは区間の中に置きます` |
| `A parameter curve needs one control per segment` | `parameter curveには区間ごとにcontrolが1つ必要です` |
| `A parameter track cannot also own an instrument or notes` | `parameter trackにはinstrumentやnoteを持たせられません` |
| `A tempo curve BPM is out of the SMF range` | `tempo curveのBPMがSMFで表せる範囲の外です` |
| `A tempo curve needs at least one tempo point` | `tempo curveにはtempoの点が1つ以上必要です` |
| `A tempo curve needs one control point per segment` | `tempo curveには区間ごとにcontrol pointが1つ必要です` |
| `A tempo curve requires one tick-zero metadata event in format 1 track 0` | `tempo curveにはformat 1のtrack 0のtick 0にmetadata eventが1つ必要です` |
| `A trill note has no Note Off` | `trillのnoteにNote Offがありません` |
| `Add pattern takes no existing pattern arguments` | `pattern追加には既存のpatternを指定しません` |
| `Ambiguous Macro {n}` | `Macro {n}が1つに決まりません` |
| `Ambiguous parameter ID: {}` | `parameter IDが1つに決まりません: {}` |
| `An unmatched Note Off lies inside a trill region` | `対応の無いNote Offがtrillの範囲の中にあります` |
| `Another command owns input` | `別のcommandが入力中です` |
| `Another editor owns input` | `別のeditorが入力中です` |
| `Arrangement blocks overlap on the same lane` | `同じlaneで配置が重なっています` |
| `Arrangement overflow` | `配置が上限を超えました` |
| `Arrangement requires PPQN timing` | `配置にはPPQNの時間単位が必要です` |
| `Audio automation: {error}` | `音声のautomation: {error}` |
| `Audio baseline restoration: {error}` | `音声の基準値の復元: {error}` |
| `Audio: plugin stopped while rendering` | `音声: 書き出し中にpluginが止まりました` |
| `Audition tempo has no pattern target` | `試聴tempoの対象patternがありません` |
| `Automation instrument changed` | `automationのinstrumentが変わりました` |
| `Automation instrument changed: {}` | `automationのinstrumentが変わりました: {}` |
| `Automation owner is outside playback scope` | `automationの持ち主が再生範囲の外です` |
| `Automation owner was not opened` | `automationの持ち主が開かれていません` |
| `Automation parameter is unavailable` | `automationのparameterを使えません` |
| `Automation requires a format 1 instrument owner track` | `automationにはformat 1のinstrumentを持つtrackが必要です` |
| `Backup sequence exhausted` | `バックアップの通し番号を使い切りました` |
| `BPM is outside the MIDI tempo range` | `BPMがMIDIのtempoで表せる範囲の外です` |
| `Bypass change must keep the chain` | `bypassの切り替えでchainを変えられません` |
| `Cache directory unavailable` | `cacheフォルダを使えません` |
| `Cannot create plugin scanner` | `plugin scannerを作れません` |
| `Cannot delete the last SMF track` | `最後のSMF trackは削除できません` |
| `Cannot kill VOICEVOX engine (PID {pid}): {}` | `VOICEVOX engine (PID {pid})を終了できません: {}` |
| `Cannot listen: {e}` | `待ち受けできません: {e}` |
| `Cannot open recording instrument` | `録音用のinstrumentを開けません` |
| `Cannot open {}: {e}` | `{}を開けません: {e}` |
| `Cannot prepare drum loop` | `drumのloopを準備できません` |
| `Cannot reach cat-plugin-player: {e}` | `cat-plugin-playerにつながりません: {e}` |
| `Cannot rename config.json to status.json: {error}` | `config.jsonをstatus.jsonへ改名できません: {error}` |
| `Cannot run netstat: {e}` | `netstatを実行できません: {e}` |
| `Cannot run taskkill: {e}` | `taskkillを実行できません: {e}` |
| `Cannot save {}: {e}` | `{}を保存できません: {e}` |
| `Cannot start player: {e}` | `playerを起動できません: {e}` |
| `Cannot start plugin scan` | `pluginのscanを始められません` |
| `Cannot start {}: {e}` | `{}を起動できません: {e}` |
| `cat-plugin-player stopped: {error}` | `cat-plugin-playerが止まりました: {error}` |
| `Chained same measure reference` | `same measureの参照先がさらに参照しています` |
| `Channel must be 1–16` | `channelは1–16で指定します` |
| `Chord guide already exists` | `chord guideは既にあります` |
| `Chord guide requires Format 1 PPQN` | `chord guideにはFormat 1のPPQNが必要です` |
| `Command target changed` | `commandの対象が変わりました` |
| `Conflicting parameter curves: {} / {}` | `parameter curveが重複しています: {} / {}` |
| `Container instrument is incompatible with {}` | `containerのinstrumentが{}に合いません` |
| `Container macro baseline domain changed` | `containerのmacro基準値の値域が変わりました` |
| `Container macro instrument changed` | `containerのmacroのinstrumentが変わりました` |
| `Could not answer the player: {error}` | `playerに応答できませんでした: {error}` |
| `Could not read the plugin state: {error}` | `pluginのstateを読めませんでした: {error}` |
| `Create a scene first` | `先にsceneを作ってください` |
| `Create at least two Macro variations first` | `先にMacroのvariationを2つ以上作ってください` |
| `Curve points must remain in tick order` | `curveの点はtick順に並べます` |
| `data before fmt` | `dataチャンクがfmtチャンクより前にあります` |
| `Delta time is too large` | `delta timeが大きすぎます` |
| `Desktop directory is unavailable` | `デスクトップフォルダを使えません` |
| `Destination contains notes` | `移動先にnoteがあります` |
| `Destination is occupied` | `移動先は使用中です` |
| `Destination parameter is unavailable` | `移動先のparameterを使えません` |
| `download exceeds size limit` | `ダウンロードが大きさの上限を超えました` |
| `Download worker stopped` | `ダウンロード処理が止まりました` |
| `Drum editing requires PPQN >= 4` | `drumの編集にはPPQN 4以上が必要です` |
| `Drum measure overflow` | `drumの小節が上限を超えました` |
| `Drum note has no Note Off` | `drumのnoteにNote Offがありません` |
| `Drum range is outside the pattern length` | `drumの範囲がpatternの長さの外です` |
| `Drum sequencer requires its four instrument rows` | `drum sequencerには4つのinstrumentの行が必要です` |
| `Drum step exceeds tick range` | `drum stepがtickの範囲を超えました` |
| `Duplicate identity across document roles` | `役割の違うdataで識別子が重複しています` |
| `Duplicate or misplaced scene metadata` | `sceneのmetadataが重複しているか置き場所が違います` |
| `Duplicate or misplaced track identity` | `trackの識別子が重複しているか置き場所が違います` |
| `Duplicate or zero arrangement identity` | `配置の識別子が重複しているか0です` |
| `Duplicate Reference Tempo tick` | `Reference Tempoのtickが重複しています` |
| `Duplicate, misplaced or oversized drum metadata` | `drumのmetadataが重複しているか、置き場所が違うか、大きすぎます` |
| `Edit exceeds the selected pattern body` | `編集が選んだpatternの本体を超えています` |
| `Edit range overflow` | `編集範囲が上限を超えました` |
| `Empty cell` | `空のcellです` |
| `Empty source cell` | `移動元のcellが空です` |
| `Enter a name, positive length and lane count` | `名前と、正の長さとlane数を入力してください` |
| `Enter a positive measure number` | `正の小節番号を入力してください` |
| `Enter a positive scene length` | `sceneの長さを正の数で入力してください` |
| `Enter meter fractions separated by spaces` | `拍子を空白区切りの分数で入力してください` |
| `expected 16-bit PCM stereo` | `16-bit PCM stereoではありません` |
| `Expected a single-line chord` | `chordは1行で入力してください` |
| `Expected channel, CC number or bend, value` | `channel・CC番号かbend・値を入力してください` |
| `Expected major or minor` | `majorかminorを入力してください` |
| `Generated state was not retained` | `生成したstateが残っていません` |
| `Initial meta events require format 1 track 0` | `初期meta eventにはformat 1のtrack 0が必要です` |
| `Instance group IDs exhausted` | `instance groupのIDを使い切りました` |
| `Instrument changed for parameter {}` | `parameter {}のinstrumentが変わりました` |
| `Invalid arrangement lane` | `配置のlaneが不正です` |
| `Invalid audition tempo` | `試聴tempoが不正です` |
| `Invalid audition tempo or PPQN timing` | `試聴tempoかPPQNの時間単位が不正です` |
| `Invalid body range` | `本体の範囲が不正です` |
| `Invalid BPM` | `BPMが不正です` |
| `Invalid container baselines` | `containerの基準値が不正です` |
| `Invalid create input` | `作成の入力が不正です` |
| `Invalid curve point` | `curveの点が不正です` |
| `Invalid dB` | `dBが不正です` |
| `Invalid denominator` | `分母が不正です` |
| `Invalid detected trill interval` | `検出したtrillの間隔が不正です` |
| `Invalid drum sequencer settings` | `drum sequencerの設定が不正です` |
| `Invalid drum step` | `drum stepが不正です` |
| `Invalid duration` | `長さが不正です` |
| `Invalid event selection` | `eventの選択が不正です` |
| `Invalid generated state size` | `生成したstateの大きさが不正です` |
| `Invalid integer octave` | `octaveの整数が不正です` |
| `Invalid integer tick` | `tickの整数が不正です` |
| `Invalid key signature (e.g. D minor)` | `調号が不正です（例: D minor）` |
| `Invalid macro values` | `macroの値が不正です` |
| `Invalid macro variation curve` | `macro variationのcurveが不正です` |
| `Invalid macro variations` | `macro variationが不正です` |
| `Invalid MBT` | `MBTが不正です` |
| `Invalid measure count` | `小節数が不正です` |
| `Invalid measure end` | `小節の終わりが不正です` |
| `Invalid measure range` | `小節の範囲が不正です` |
| `Invalid measure start` | `小節の始まりが不正です` |
| `Invalid MIDI baseline value` | `MIDIの基準値が不正です` |
| `Invalid MIDI channel` | `MIDI channelが不正です` |
| `Invalid note` | `noteが不正です` |
| `Invalid note name` | `音名が不正です` |
| `Invalid numerator` | `分子が不正です` |
| `Invalid or duplicate fixed macro target` | `固定macroの対象が不正か重複しています` |
| `Invalid or duplicate instance group identity` | `instance groupの識別子が不正か重複しています` |
| `Invalid or duplicate MIDI baseline` | `MIDIの基準値が不正か重複しています` |
| `Invalid or duplicate parameter baseline` | `parameterの基準値が不正か重複しています` |
| `Invalid or duplicate scene cell` | `sceneのcellが不正か重複しています` |
| `Invalid or duplicate track volume metadata` | `trackの音量のmetadataが不正か重複しています` |
| `Invalid parameter curve version or target identity` | `parameter curveのversionか対象の識別子が不正です` |
| `Invalid pattern curve ownership or range` | `pattern curveの持ち主か範囲が不正です` |
| `Invalid pattern identity or container` | `patternの識別子かcontainerが不正です` |
| `Invalid pattern meter` | `patternの拍子が不正です` |
| `Invalid pattern version or identity` | `patternのversionか識別子が不正です` |
| `Invalid performance owner` | `演奏表現の持ち主が不正です` |
| `Invalid performance parameter window` | `演奏表現のparameterの範囲が不正です` |
| `Invalid performance parameter: {e}` | `演奏表現のparameterが不正です: {e}` |
| `Invalid Reference Tempo metadata` | `Reference Tempoのmetadataが不正です` |
| `Invalid Reference Tempo rank` | `Reference Tempoの順位が不正です` |
| `invalid release entry` | `releaseの項目が不正です` |
| `Invalid same measure payload` | `same measureの中身が不正です` |
| `Invalid scene identity or length` | `sceneの識別子か長さが不正です` |
| `Invalid scene metadata: {e}` | `sceneのmetadataが不正です: {e}` |
| `Invalid scene meter` | `sceneの拍子が不正です` |
| `Invalid scene version` | `sceneのversionが不正です` |
| `Invalid Song lane` | `Songのlaneが不正です` |
| `Invalid Song placement` | `Songの配置が不正です` |
| `Invalid tempo event` | `tempo eventが不正です` |
| `Invalid tick` | `tickが不正です` |
| `Invalid ticks per quarter note` | `四分音符あたりのtick数が不正です` |
| `Invalid Trill End metadata` | `Trill Endのmetadataが不正です` |
| `Invalid Trill Start metadata` | `Trill Startのmetadataが不正です` |
| `Invalid trill tempo event` | `trillのtempo eventが不正です` |
| `Invalid variation instrument identity` | `variationのinstrumentの識別子が不正です` |
| `Invalid velocity` | `velocityが不正です` |
| `Invalid zero tempo` | `tempoが0で不正です` |
| `Invalid zero trill tempo` | `trillのtempoが0で不正です` |
| `Key signature must use at most seven sharps or flats` | `調号のsharp・flatは7個までです` |
| `Lane requires exactly one owner` | `laneには持ち主がちょうど1つ必要です` |
| `Layer ID exhausted` | `layerのIDを使い切りました` |
| `Layer selection target no longer exists or is active` | `layerの選択先が無くなったか、既に有効です` |
| `LOCALAPPDATA is unavailable` | `LOCALAPPDATAを使えません` |
| `Loop recording requires at least one measure` | `Loop録音には1小節以上が必要です` |
| `Macro values are too large` | `macroの値が大きすぎます` |
| `Macro variation curve instrument changed` | `macro variation curveのinstrumentが変わりました` |
| `Macro variation curve references a missing variation` | `macro variation curveが無いvariationを参照しています` |
| `Macro variation differs from active values` | `macro variationが今の値と違います` |
| `Macro variation instrument changed` | `macro variationのinstrumentが変わりました` |
| `Macro variation targets differ` | `macro variationの対象が違います` |
| `Malformed pattern MIDI control` | `patternのMIDI controlが壊れています` |
| `Malformed pattern note` | `patternのnoteが壊れています` |
| `Managed trill notes use the Trill interval` | `管理されたtrillのnoteはTrillの間隔で変えます` |
| `Measure exceeds tick range` | `小節がtickの範囲を超えました` |
| `Measure is outside the pattern length` | `小節がpatternの長さの外です` |
| `Measure lengths differ` | `小節の長さが違います` |
| `Measure must be positive` | `小節は正の数で指定します` |
| `Measure number must be positive` | `小節番号は正の数で指定します` |
| `Meter cannot be represented by whole ticks` | `拍子を整数のtickで表せません` |
| `Misplaced or duplicate macro values` | `macroの値の置き場所が違うか重複しています` |
| `Misplaced or duplicate macro variation curves` | `macro variation curveの置き場所が違うか重複しています` |
| `Missing arrangement pattern` | `配置のpatternが見つかりません` |
| `Missing automation layer: {}` | `automationのlayerが見つかりません: {}` |
| `Missing automation owner` | `automationの持ち主が見つかりません` |
| `Missing conductor` | `conductorが見つかりません` |
| `Missing conductor track` | `conductor trackが見つかりません` |
| `Missing container baseline for channel {} {}` | `channel {} {}のcontainer基準値が見つかりません` |
| `Missing container macro layer` | `containerのmacro layerが見つかりません` |
| `Missing container parameter baseline` | `containerのparameter基準値が見つかりません` |
| `Missing container patch` | `containerのpatchが見つかりません` |
| `Missing created pattern` | `作ったpatternが見つかりません` |
| `Missing destination container` | `移動先のcontainerが見つかりません` |
| `Missing destination sound` | `移動先のsoundが見つかりません` |
| `Missing drum container` | `drum containerが見つかりません` |
| `Missing drum measure map` | `drumの小節の対応表が見つかりません` |
| `Missing drum track` | `drum trackが見つかりません` |
| `Missing edit track` | `編集中のtrackが見つかりません` |
| `Missing fixed macro baseline` | `固定macroの基準値が見つかりません` |
| `Missing layer` | `layerが見つかりません` |
| `Missing layer for parameter {}` | `parameter {}のlayerが見つかりません` |
| `Missing macro owner` | `macroの持ち主が見つかりません` |
| `Missing macro variations for curve` | `curveのmacro variationが見つかりません` |
| `Missing Macro {n} parameter` | `Macro {n}のparameterが見つかりません` |
| `Missing Macro {n} saved position` | `Macro {n}の保存位置が見つかりません` |
| `Missing measure meter` | `小節の拍子が見つかりません` |
| `Missing meter` | `拍子が見つかりません` |
| `Missing or incompatible container parameter baseline` | `containerのparameter基準値が見つからないか合いません` |
| `Missing or invalid scene lane` | `sceneのlaneが見つからないか不正です` |
| `Missing owner for parameter {}` | `parameter {}の持ち主が見つかりません` |
| `Missing owner: {} / {}` | `持ち主が見つかりません: {} / {}` |
| `Missing parameter baseline` | `parameterの基準値が見つかりません` |
| `Missing parameter: {} / {}` | `parameterが見つかりません: {} / {}` |
| `Missing pattern edit base` | `patternの編集の基準が見つかりません` |
| `Missing pattern occurrence` | `patternの出現箇所が見つかりません` |
| `Missing pattern parameter` | `patternのparameterが見つかりません` |
| `Missing placement lane or measure` | `配置のlaneか小節が見つかりません` |
| `Missing preview pattern` | `試聴するpatternが見つかりません` |
| `Missing recorded pattern` | `録音したpatternが見つかりません` |
| `Missing Scene` | `Sceneが見つかりません` |
| `Missing scene cell` | `sceneのcellが見つかりません` |
| `Missing song meter` | `songの拍子が見つかりません` |
| `Missing sound owner` | `soundの持ち主が見つかりません` |
| `Missing trill pattern` | `trillのpatternが見つかりません` |
| `Missing Trill Start` | `Trill Startが見つかりません` |
| `Missing {prefix} saved position` | `{prefix}の保存位置が見つかりません` |
| `MML exceeds the pattern measure limit` | `MMLがpatternの小節数の上限を超えました` |
| `MML insertion requires ticks per quarter note` | `MMLの挿入には四分音符あたりのtick数が必要です` |
| `MML starts before the pattern` | `MMLがpatternより前から始まっています` |
| `Multiple references in destination` | `移動先に参照が複数あります` |
| `Multiple tracks share this parameter target` | `複数のtrackがこのparameterを対象にしています` |
| `No continuous automatable parameters` | `連続的にautomationできるparameterがありません` |
| `no data chunk` | `dataチャンクがありません` |
| `No executable directory` | `実行ファイルのフォルダがありません` |
| `No guide event selected` | `guide eventを選んでいません` |
| `No input` | `入力がありません` |
| `No installed {} patches in the catalog` | `catalogにinstall済みの{} patchがありません` |
| `No instrument` | `instrumentがありません` |
| `No local application directory` | `local applicationフォルダがありません` |
| `No matching parameter` | `合うparameterがありません` |
| `No measure selection` | `小節を選んでいません` |
| `No MIDI document is loaded` | `MIDIが開かれていません` |
| `No numeric field selected` | `数値の欄を選んでいません` |
| `No parameter selection` | `parameterを選んでいません` |
| `No Reference Tempo at this tick` | `このtickにReference Tempoがありません` |
| `No routed macros` | `routeされたmacroがありません` |
| `No selected layer` | `layerを選んでいません` |
| `No selected point` | `点を選んでいません` |
| `No SMF track is loaded` | `SMF trackが開かれていません` |
| `No such curve point` | `そのcurveの点はありません` |
| `No such effect slot` | `そのeffect slotはありません` |
| `No such parameter control` | `そのparameter controlはありません` |
| `No such parameter segment` | `そのparameterの区間はありません` |
| `No value input` | `値の入力がありません` |
| `No value is being edited` | `編集中の値がありません` |
| `not RIFF/WAVE` | `RIFF/WAVEではありません` |
| `Note crosses pattern boundary` | `noteがpatternの境界をまたいでいます` |
| `Note edit target changed` | `noteの編集対象が変わりました` |
| `Note has no matching end` | `noteに対応する終わりがありません` |
| `Overlapping pattern control {control:?}` | `patternのcontrolが重なっています {control:?}` |
| `Overlapping performance parameter windows` | `演奏表現のparameterの範囲が重なっています` |
| `Owner track {} is unavailable` | `持ち主のtrack {}を使えません` |
| `Parameter automation requires SMF format 1` | `parameterのautomationにはSMF format 1が必要です` |
| `Parameter format/value space mismatch` | `parameterの形式と値域が合いません` |
| `Parameter instrument is unavailable` | `parameterのinstrumentを使えません` |
| `Parameter is missing or not automatable: {} / {}` | `parameterが見つからないかautomationできません: {} / {}` |
| `Parameter metadata is too large` | `parameterのmetadataが大きすぎます` |
| `Parameter metadata requires tick zero in SMF format 1` | `parameterのmetadataはSMF format 1のtick 0に置きます` |
| `Parameter points must have unique increasing ticks` | `parameterの点は重複の無い昇順のtickに置きます` |
| `Parameter range or value space changed: {} / {}` | `parameterの範囲か値域が変わりました: {} / {}` |
| `Parameter target changed during editing` | `編集中にparameterの対象が変わりました` |
| `Patch or plugin changed during state generation` | `state生成中にpatchかpluginが変わりました` |
| `Patch state requires session restoration` | `patchのstateにはsessionの復元が必要です` |
| `Pattern and Song meters differ` | `patternとSongの拍子が違います` |
| `Pattern contains an unmatched or carried note off` | `patternに対応の無いnote offか持ち越したnote offがあります` |
| `Pattern contains an unterminated note` | `patternに終わりの無いnoteがあります` |
| `Pattern contains same measure reference` | `patternにsame measureの参照があります` |
| `Pattern content extends beyond the requested length` | `patternの中身が指定の長さを超えています` |
| `Pattern curve time overflow` | `pattern curveの時間が上限を超えました` |
| `Pattern curves must be stored at the body start` | `pattern curveは本体の始まりに置きます` |
| `Pattern drum-row interpretation is incompatible with the destination container` | `patternのdrumの行の解釈が移動先のcontainerに合いません` |
| `Pattern edit has no placement` | `編集中のpatternに配置がありません` |
| `Pattern edit target changed` | `patternの編集対象が変わりました` |
| `Pattern event overflow` | `patternのeventが上限を超えました` |
| `Pattern input target changed` | `patternの入力先が変わりました` |
| `Pattern is in use` | `patternは使用中です` |
| `Pattern length overflow` | `patternの長さが上限を超えました` |
| `Pattern meter does not match its exact length` | `patternの拍子が長さと合いません` |
| `Pattern occurrence exceeds the declared length` | `patternの出現箇所が宣言した長さを超えています` |
| `Pattern occurrence overflow` | `patternの出現箇所が上限を超えました` |
| `Pattern requires an explicit length` | `patternには長さの指定が必要です` |
| `Pattern storage ranges overlap or have no boundary gap` | `patternの保存範囲が重なっているか境界の隙間がありません` |
| `Performance export must use a different file from the editable MID` | `演奏用の出力先には編集用のMIDと別のファイルを指定します` |
| `Performance expression windows cannot share an owner with ordinary parameter tracks` | `演奏表現の範囲は通常のparameter trackと持ち主を共有できません` |
| `Pitch must be 0–127` | `音高は0–127で指定します` |
| `Placement exceeds the declared Song length` | `配置が宣言したSongの長さを超えています` |
| `Player restoration failed: {}` | `playerの復元に失敗しました: {}` |
| `Player returned an invalid state size` | `playerが返したstateの大きさが不正です` |
| `Player root discovery failed: {}` | `playerのroot探索に失敗しました: {}` |
| `Player stdin unavailable` | `playerの標準入力を使えません` |
| `Plugin state has no restoration reference` | `pluginのstateに復元の参照がありません` |
| `Prefetch worker stopped` | `先読み処理が止まりました` |
| `Program Change cannot change a pattern container's sound` | `Program Changeではpattern containerのsoundを変えられません` |
| `Published cat-plugin-player is too old for this editor (requires --resident, --restore-state and --patch-roots). A newer player release is required.` | `公開中のcat-plugin-playerはこのeditorには古すぎます（--resident・--restore-state・--patch-rootsが必要）。新しいplayerのreleaseが必要です。` |
| `Recording container disappeared` | `録音先のcontainerが無くなりました` |
| `Recording division changed` | `録音の分割が変わりました` |
| `Recording has held notes` | `録音中に押さえたままのnoteがあります` |
| `Recording has not started` | `録音が始まっていません` |
| `Recording storage overflow` | `録音の保存領域が上限を超えました` |
| `Reference must be at measure start` | `参照は小節の始まりに置きます` |
| `release is not a Windows executable` | `releaseがWindowsの実行ファイルではありません` |
| `Render stopped` | `書き出しが止まりました` |
| `Same measure number must be positive` | `same measureの小節番号は正の数で指定します` |
| `Scene and song measure lengths differ` | `sceneとsongの小節の長さが違います` |
| `Scene edit placement changed` | `編集中のsceneの配置が変わりました` |
| `Scene is in use` | `sceneは使用中です` |
| `Scene measure is outside its length` | `sceneの小節が長さの外です` |
| `Scene meter cannot represent its exact measure boundaries` | `sceneの拍子で小節の境界を正確に表せません` |
| `Scene pattern measure lengths differ` | `sceneのpatternの小節の長さが違います` |
| `Scene placement outside its lanes` | `sceneの配置がlaneの外です` |
| `Scene placement overflow` | `sceneの配置が上限を超えました` |
| `Scene requires PPQN timing` | `sceneにはPPQNの時間単位が必要です` |
| `Scenes contain patterns only` | `sceneに入れられるのはpatternだけです` |
| `Select a drum sound container` | `drum sound containerを選んでください` |
| `Select a guide event` | `guide eventを選んでください` |
| `Select a key or chord` | `keyかchordを選んでください` |
| `Select a normal track` | `通常のtrackを選んでください` |
| `Select a note track` | `note trackを選んでください` |
| `Select a pattern` | `patternを選んでください` |
| `Select a Scene` | `Sceneを選んでください` |
| `Select a sound container` | `sound containerを選んでください` |
| `Select an instrument owner track` | `instrumentを持つtrackを選んでください` |
| `Select an instrument track` | `instrument trackを選んでください` |
| `Selection target is no longer loaded` | `選択先がもう開かれていません` |
| `Self reference` | `自分自身を参照しています` |
| `ShellExecuteW failed ({result})` | `ShellExecuteWに失敗しました ({result})` |
| `SMF directory is missing: {}` | `SMFのフォルダがありません: {}` |
| `SMF events are out of tick order` | `SMFのeventがtick順に並んでいません` |
| `SMF filename is missing` | `SMFのファイル名がありません` |
| `Song changed` | `Songが変わりました` |
| `Song has no explicit performance length` | `Songに演奏の長さの指定がありません` |
| `Source and destination overlap` | `移動元と移動先が重なっています` |
| `State cache {}: {e}` | `stateのcache {}: {e}` |
| `State/reference plugin mismatch` | `stateと参照のpluginが合いません` |
| `Symlinked plugin bundles cannot be cached safely` | `symlinkのplugin bundleは安全にcacheできません` |
| `Tempo points must be in increasing tick order` | `tempoの点はtickの昇順に並べます` |
| `The duration is too large` | `長さが大きすぎます` |
| `The MIDI document changed` | `MIDIが変わりました` |
| `The move crosses another note of the same key` | `移動すると同じ音高の別のnoteと重なります` |
| `The note cannot move before tick 0 or that far` | `noteはtick 0より前やそこまで遠くへは動かせません` |
| `The phrase's Capture pattern no longer exists` | `phraseのCapture patternが無くなりました` |
| `The phrase's container no longer exists` | `phraseのcontainerが無くなりました` |
| `The resident player requires Windows` | `常駐playerにはWindowsが必要です` |
| `The start cannot move before tick 0` | `始まりはtick 0より前へ動かせません` |
| `The start must be before the Note Off` | `始まりはNote Offより前に置きます` |
| `This pattern action has no container input` | `このpattern操作にはcontainerの入力がありません` |
| `This pattern action has no value input` | `このpattern操作には値の入力がありません` |
| `Too many automated parameters on one layer` | `1つのlayerでautomationするparameterが多すぎます` |
| `Too many instrument layers` | `instrument layerが多すぎます` |
| `Too many macro variation sets` | `macro variationの組が多すぎます` |
| `Track has no instrument layer` | `trackにinstrument layerがありません` |
| `Track has no layers` | `trackにlayerがありません` |
| `Track {}, tick {}: {error}` | `Track {}・tick {}: {error}` |
| `Trill detection requires format 1 with PPQN timing` | `trillの検出にはformat 1とPPQNの時間単位が必要です` |
| `Trill durations must be finite and positive` | `trillの長さは有限の正の数で指定します` |
| `Trill end tick overflow` | `trillの終わりのtickが上限を超えました` |
| `Trill interval must be finite and positive` | `trillの間隔は有限の正の数で指定します` |
| `Trill metadata requires format 1 with PPQN timing` | `trillのmetadataにはformat 1とPPQNの時間単位が必要です` |
| `Trill notes overlap or cross their region boundary` | `trillのnoteが重なっているか範囲の境界をまたいでいます` |
| `Trill regeneration exceeds one million notes` | `trillの再生成が100万noteを超えました` |
| `Trill regions overlap on the same channel and pitch` | `同じchannelと音高でtrillの範囲が重なっています` |
| `Trill tempo curve requires tempos only in the conductor` | `trillのtempo curveではtempoをconductorだけに置きます` |
| `truncated data` | `dataが途中で切れています` |
| `Unknown event` | `不明なeventです` |
| `Unknown note` | `不明なnoteです` |
| `Unmatched Trill End` | `対応の無いTrill Endです` |
| `Unmatched trill metadata` | `対応の無いtrillのmetadataです` |
| `Unmatched Trill Start` | `対応の無いTrill Startです` |
| `Unrepresentable audition tempo` | `試聴tempoを表せません` |
| `Unsupported CC baseline` | `対応していないCCの基準値です` |
| `Unsupported measure length` | `対応していない小節の長さです` |
| `Unsupported patch catalog version` | `対応していないpatch catalogのversionです` |
| `Unsupported tempo curve schema` | `対応していないtempo curveの形式です` |
| `Velocity must be 1–127` | `velocityは1–127で指定します` |
| `VOICEVOX /sing_frame_audio_query: not an object` | `VOICEVOX /sing_frame_audio_query: objectではありません` |
| `VOICEVOX cannot automate plugin parameters: {}` | `VOICEVOXではpluginのparameterをautomationできません: {}` |
| `VOICEVOX voice unavailable: {}` | `VOICEVOXの声を使えません: {}` |
| `VOICEVOX WAV: {actual} Hz, expected {rate} Hz` | `VOICEVOX WAV: {actual} Hzです（{rate} Hzのはず）` |
| `VOICEVOX: catalog thread panicked` | `VOICEVOX: catalogの処理が異常終了しました` |
| `VOICEVOX: no cache directory` | `VOICEVOX: cacheフォルダがありません` |
| `VOICEVOX: no humming voices` | `VOICEVOX: hummingの声がありません` |
| `VOICEVOX: prerender thread panicked` | `VOICEVOX: 事前合成の処理が異常終了しました` |
| `VOICEVOX: sample thread panicked` | `VOICEVOX: 試聴の処理が異常終了しました` |
| `WAV path has no directory` | `WAVのpathにフォルダがありません` |
| `{} did not answer on port {port} in {} s` | `{}がport {port}で{} s以内に応答しませんでした` |
| `{} exited before answering ({status})` | `{}が応答する前に終了しました ({status})` |
| `{} is already connected in another slot` | `{}は既に別のslotにつながっています` |

## 抽出スクリプトが拾わなかった文

抽出スクリプトは次の文を拾わない。どれも画面に出るので、上と同じ規則で訳した。2か所以上で使う文は `src/message.rs` に置いた（「集めた文の訳」に含めた）。

- 文字列の中の `;` で区切りを打ち切る（例: `Parameter target changed; reopen Automation`）。
- エラー文脈の一覧に無い書き方（`error: Some(`・`unwrap_or(`・`warnings.push(`・`let invalid = |e| format!(`）。
- エラー文脈に掛からない表示用の `"invalid"`（SMF の meta event の中身を説明する文）。

| 原文 | 訳 | 使用箇所 |
|---|---|---|
| `Audio parameter batch failed ({code}): {}` | `音声: parameterの一括設定に失敗しました ({code}): {}` | src/play/audition_pump.rs |
| `unknown target` | `不明な対象` | src/play/audition_pump.rs |
| `Audio: plugin or audio device stopped` | `音声: pluginか音声デバイスが止まりました` | src/play/audition_pump.rs |
| `Cannot open recording instrument` | `message::CANNOT_OPEN_RECORDING_INSTRUMENT` | src/view/recording/play.rs, src/view/recording/phrase.rs |
| `Cannot open {OFFICIAL_PAGE}: {e}` | `{OFFICIAL_PAGE}を開けません: {e}` | src/view/voicevox.rs |
| `Cannot save session: LOCALAPPDATA is unavailable` | `sessionを保存できません: LOCALAPPDATAを使えません` | src/store/config.rs |
| `Cannot save recent files: LOCALAPPDATA is unavailable` | `最近使ったファイルを保存できません: LOCALAPPDATAを使えません` | src/store/recent.rs |
| `Cannot save {}: {error}` | `message::cannot_save` | src/store/config.rs, src/view/selection_history.rs, src/store/saved.rs |
| `Invalid time signature: track {}, event {} (tick {})` | `拍子が不正です: track {}・event {} (tick {})` | src/score/musical_time.rs |
| `Conflicting time signatures at tick {tick}; last track/event wins` | `tick {tick}で拍子が食い違っています。最後のtrack/eventを使います` | src/score/musical_time.rs |
| `Invalid pattern meta: {e}` | `pattern metaが不正です: {e}` | src/score/pattern.rs |
| `CC {cc} requires an unsupported compound/channel state machine; content retained` | `CC {cc}は対応していない複合/channelの状態遷移を要します。内容はそのまま残します` | src/score/pattern/render.rs |
| `Pattern SysEx performance is unsupported; content retained` | `patternのSysEx演奏には対応していません。内容はそのまま残します` | src/score/pattern/render.rs |
| `Pattern aftertouch performance is unsupported; content retained` | `patternのaftertouch演奏には対応していません。内容はそのまま残します` | src/score/pattern/render.rs |
| `Container macro variation curves are not pattern expressions; content retained` | `containerのmacro variation curveはpatternの表現ではありません。内容はそのまま残します` | src/score/performance.rs |
| `Parameter target changed; reopen Automation` | `message::PARAMETER_TARGET_CHANGED` | src/view/parameter_editor.rs, src/view/parameter_editor/preview.rs |
| `Parameter instance changed; reopen Automation` | `message::PARAMETER_INSTANCE_CHANGED` | src/view/parameter_editor.rs（2か所） |
| `Macro target changed; reopen Macro edit` | `macroの対象が変わりました。Macro editを開き直してください` | src/view/macro_editor.rs |
| `Selection target changed while the browser was open; result discarded` | `browserを開いている間に選択先が変わったので、結果を捨てました` | src/player/resident.rs |
| `MIDI file changed since it was loaded; refusing to overwrite` | `MIDIファイルが読み込んだ後に変わったので、上書きしません` | src/store/saved_write.rs |
| `Duplicate retains its container; move it explicitly afterward` | `複製は元のcontainerのままです。後から移動してください` | src/score/pattern/operations.rs |
| `invalid` | `message::INVALID` | src/score/event_content/sequencer.rs（9か所）, src/score/pattern.rs, src/score/performance.rs, src/score/pattern/automation.rs（2か所）, src/score/scene.rs |

## 書式だけ（訳さない）

| 文 | 使用箇所数 | 使用箇所 |
|---|---:|---|
| `{}: {error}` | 3 | src/app/loading.rs:9, src/app/loading.rs:22, src/view/opening.rs:76 |
| `VOICEVOX: {error}` | 1 | src/play/sung/prerender.rs:202 |
| `cat-plugin-player: {e}` | 1 | src/player/setup/download.rs:20 |
| `VOICEVOX /version: {e}` | 1 | src/voicevox/client.rs:34 |
| `VOICEVOX /singers: {e}` | 2 | src/voicevox/client.rs:52, src/voicevox/client.rs:132 |
| `VOICEVOX /sing_frame_audio_query: {e}` | 1 | src/voicevox/client.rs:65 |
| `VOICEVOX {path}: {e}` | 3 | src/voicevox/client.rs:83, src/voicevox/client.rs:92, src/voicevox/client.rs:108 |
| `VOICEVOX {path}: {status} {}` | 1 | src/voicevox/client.rs:95 |
| `VOICEVOX {path}: {status} {text}` | 1 | src/voicevox/client.rs:112 |

## 除外した使用箇所

| 使用箇所 | 文 | 区分 |
|---|---|---|
| src/main.rs:20 | `startup` | 誤検出 |
| src/main.rs:33 | `arguments` | 誤検出 |
| src/main.rs:39 | `Cache directory unavailable` | ログのみ |
| src/main.rs:44 | `surge-data-home` | 誤検出 |
| src/main.rs:54 | `LOCALAPPDATA is unavailable` | ログのみ |
| src/play/mascot/server.rs:115 | `{}: {error}` | ログのみ |
| src/play/audition_probe.rs:45 | `Probe refuses to generate or replace cached patch state` | テスト専用 |
| src/play/mascot.rs:28 | `mascot` | 誤検出 |
| src/play/mascot/server.rs:58 | `mascot` | 誤検出 |
| src/play/mascot/server.rs:18 | `{error:#}` | ログのみ |
| src/play/mascot/server.rs:25 | `{error:#}` | ログのみ |
| src/play/mascot/server.rs:95 | `mascot-render-server executable not found: {}` | ログのみ |
| src/player/setup.rs:37 | `PATH` | 誤検出 |
| src/player/setup.rs:115 | `setup` | 誤検出 |
| src/startup_trace/native_audio/instance.rs:17 | `Negative instance ID` | テスト専用 |
| src/store/cli.rs:24 | `Unknown argument: {}` | ログのみ |
| src/store/cli.rs:27 | `Only one MIDI file may be specified` | ログのみ |
| src/store/cli.rs:33 | `--player may only be specified once` | ログのみ |
| src/store/cli.rs:35 | `--player requires a path` | ログのみ |
| src/store/cli.rs:37 | `--player requires a path` | ログのみ |
| src/store/error_log.rs:24 | `Cannot write {}: {error}` | ログのみ |
| src/store/settings.rs:42 | `window requires finite x/y and positive width/height` | ログのみ |
| src/view/files.rs:165 | `backup` | 誤検出 |
| src/view/files/performance.rs:49 | `backup` | 誤検出 |
| src/view/commands/boundary.rs:75 | `Palette domain dependency: {token}` | テスト専用 |
| src/view/commands/boundary.rs:82 | `Palette interprets command IDs` | テスト専用 |
| src/view/commands/boundary.rs:89 | `Palette compares command IDs` | テスト専用 |
| src/view/commands/boundary.rs:96 | `Palette dispatches on command IDs` | テスト専用 |
| src/view/macro_editor/preview.rs:12 | `Macros` | 誤検出 |
| src/view/macro_editor/ui.rs:197 | `Plugin ? · API read ?` | 誤検出 |
