//! 画面に出るエラー文のうち、2か所以上で使う文。1か所でしか使わない文は使う場所に書く。

use std::fmt::{Debug, Display};

pub const ARRANGEMENT_INPUT_TARGET_CHANGED: &str = "配置の入力先が変わりました";
pub const AUDIO_DEVICE_DID_NOT_START_PLAYBACK: &str = "音声デバイスが再生を始めませんでした";
pub const A_TEMPO_CURVE_REQUIRES_SMF_FORMAT_1: &str = "tempo curveにはSMF format 1が必要です";
pub const CANNOT_OPEN_RECORDING_INSTRUMENT: &str = "録音用のinstrumentを開けません";
pub const BACKUP_DIRECTORY_IS_UNAVAILABLE: &str = "バックアップフォルダを使えません";
pub const DRUM_STEP_OVERFLOW: &str = "drum stepが上限を超えました";
pub const EDIT_PATTERN_FROM_DESTINATION_TRACK: &str = "patternは配置先のtrackで編集してください";
pub const INVALID: &str = "不正";
pub const INVALID_DESTINATION_LANE: &str = "移動先のlaneが不正です";
pub const INVALID_DRUM_ROW: &str = "drumの行が不正です";
pub const INVALID_EVENT: &str = "eventが不正です";
pub const INVALID_REFERENCE_TEMPO_BPM: &str = "Reference TempoのBPMが不正です";
pub const INVALID_SCENE_MEASURE: &str = "sceneの小節が不正です";
pub const INVALID_TRACK: &str = "trackが不正です";
pub const INVALID_TRACK_VOLUME: &str = "trackの音量が不正です";
pub const LAYER_WAS_REMOVED: &str = "layerが削除されました";
pub const LOOP_IS_NOT_PLACED: &str = "Loopは配置できません";
pub const MASTER_EFFECTS_REQUIRE_SMF_FORMAT_1: &str =
    "Master effectsの編集にはSMF format 1が必要です";
pub const MEASURE_OVERFLOW: &str = "小節が上限を超えました";
pub const MIDI_NOTE_MUST_BE_0_127: &str = "MIDI noteは0–127で指定します";
pub const MISSING_CONTAINER: &str = "containerが見つかりません";
pub const MISSING_CONTAINER_SOUND: &str = "container soundが見つかりません";
pub const MISSING_DEFINITION: &str = "定義が見つかりません";
pub const MISSING_EDIT_SCENE: &str = "編集中のsceneが見つかりません";
pub const MISSING_INSTRUMENT_LAYER: &str = "instrument layerが見つかりません";
pub const MISSING_MEASURE_END: &str = "小節の終わりが見つかりません";
pub const MISSING_MEASURE_MAP: &str = "小節の対応表が見つかりません";
pub const MISSING_MEASURE_START: &str = "小節の始まりが見つかりません";
pub const MISSING_PATTERN: &str = "patternが見つかりません";
pub const MISSING_PATTERN_METER: &str = "patternの拍子が見つかりません";
pub const MISSING_PLACEMENT: &str = "配置が見つかりません";
pub const MISSING_SCENE: &str = "sceneが見つかりません";
pub const MISSING_SCENE_PATTERN: &str = "sceneのpatternが見つかりません";
pub const MISSING_SONG_METER: &str = "Songの拍子が見つかりません";
pub const MISSING_SOUND_CONTAINER: &str = "sound containerが見つかりません";
pub const MISSING_TRACK: &str = "trackが見つかりません";
pub const NOTE_CROSSES_MEASURE_BOUNDARY: &str = "noteが小節をまたいでいます";
pub const NOT_A_NOTE_ON: &str = "Note Onではありません";
pub const NOT_A_PARAMETER_TRACK: &str = "parameter trackではありません";
pub const NO_AUDIO_OUTPUT_DEVICE: &str = "音声出力デバイスがありません";
pub const NO_DRUM_SETTINGS: &str = "drum設定がありません";
pub const NO_NOTES: &str = "noteがありません";
pub const NO_SMF_LOADED: &str = "SMFが開かれていません";
pub const NO_SONG: &str = "Songがありません";
pub const NO_SUCH_TRACK: &str = "そのtrackはありません";
pub const PARAMETER_CONTROL_MUST_BE_FINITE: &str = "parameterの値は有限の数で指定します";
pub const PARAMETER_DISAPPEARED: &str = "parameterが無くなりました";
pub const PARAMETER_INSTRUMENT_UNAVAILABLE: &str = "parameterのinstrumentを使えません";
pub const PARAMETER_OWNER_CHANGED: &str = "parameterの持ち主が変わりました";
pub const PARAMETER_VALUE_IS_OUTSIDE_ITS_NATIVE_RANGE: &str = "parameterの値が本来の範囲の外です";
pub const PARAMETER_INSTANCE_CHANGED: &str =
    "parameterのinstanceが変わりました。Automationを開き直してください";
pub const PARAMETER_TARGET_CHANGED: &str =
    "parameterの対象が変わりました。Automationを開き直してください";
pub const PATTERN_EDIT_TARGET_NO_LONGER_EXISTS: &str = "patternの編集対象が無くなりました";
pub const PATTERN_RANGE_OVERFLOW: &str = "patternの範囲が上限を超えました";
pub const PATTERN_REQUIRES_PPQN_TIMING: &str = "patternにはPPQNの時間単位が必要です";
pub const RECORDING_EXCEEDS_THE_PATTERN_MEASURE_LIMIT: &str =
    "録音がpatternの小節数の上限を超えました";
pub const RECORDING_IS_EMPTY: &str = "録音が空です";
pub const RECORDING_REQUIRES_A_POSITIVE_TEMPO: &str = "録音には正のtempoが必要です";
pub const RECORDING_REQUIRES_PPQN_TIMING: &str = "録音にはPPQNの時間単位が必要です";
pub const RECORDING_TIME_OVERFLOW: &str = "録音時間が上限を超えました";
pub const REFERENCE_TEMPO_REQUIRES_FORMAT_1_TRACK_0: &str =
    "Reference Tempoにはformat 1のtrack 0が必要です";
pub const SAME_MEASURE_REQUIRES_PPQN_TIMING: &str = "same measureにはPPQNの時間単位が必要です";
pub const SCENE_LENGTH_OVERFLOW: &str = "sceneの長さが上限を超えました";
pub const SELECT_A_CONTAINER: &str = "containerを選んでください";
pub const SELECT_A_PATTERN_OR_SCENE: &str = "patternかsceneを選んでください";
pub const SELECT_A_SCENE: &str = "sceneを選んでください";
pub const SELECT_THE_CHORD_GUIDE_TRACK: &str = "chord guide trackを選んでください";
pub const STORAGE_OVERFLOW: &str = "保存領域が上限を超えました";
pub const THE_DURATION_MUST_BE_AT_LEAST_1_TICK: &str = "長さは1 tick以上で指定します";
pub const THIS_NOTE_HAS_NO_NOTE_OFF: &str = "このnoteにはNote Offがありません";
pub const THIS_PATCH_HAS_NO_ROUTED_MACROS: &str = "このpatchにはrouteされたmacroがありません";
pub const TICK_OVERFLOW: &str = "tickが上限を超えました";
pub const TOO_MANY_SMF_TRACKS: &str = "SMFのtrackが多すぎます";
pub const TRACK_WAS_REMOVED: &str = "trackが削除されました";
pub const TRILL_INTERVAL_BELOW_TICK_RESOLUTION: &str = "trillの間隔がtickの分解能より短くなります";
pub const UNKNOWN_TRILL: &str = "不明なtrillです";
pub const VOICEVOX_ENGINE_IS_NOT_SET: &str = "VOICEVOX engineが設定されていません";

pub fn audio(error: impl Display) -> String {
    format!("音声: {error}")
}

pub fn audio_automation_refresh(error: impl Display) -> String {
    format!("音声のautomation更新: {error}")
}

pub fn automation(error: impl Display) -> String {
    format!("automationエラー: {error}")
}

pub fn cannot_inspect_macro_routing(instrument: impl Display) -> String {
    format!("{instrument}のmacro routingを調べられません")
}

pub fn cannot_read(path: impl Display, error: impl Display) -> String {
    format!("{path}を読めません: {error}")
}

pub fn cannot_save(path: impl Display, error: impl Display) -> String {
    format!("{path}を保存できません: {error}")
}

pub fn cannot_scan(path: impl Display, error: impl Display) -> String {
    format!("{path}の中を一覧できません: {error}")
}

pub fn invalid_value(error: impl Display) -> String {
    format!("不正 ({error})")
}

pub fn master(error: impl Display) -> String {
    format!("Master: {error}")
}

pub fn track_error(number: impl Display, error: impl Display) -> String {
    format!("Track {number}: {error}")
}

pub fn unsupported_audio_format(format: impl Debug) -> String {
    format!("対応していない音声形式です: {format:?}")
}
