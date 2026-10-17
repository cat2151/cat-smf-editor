# 0002: track別音源と最小パッチブラウザ

Surge専用host・factory path保存・簡易browserと取消契約は、[0003: 共通hostとplayer選択](0003-shared-host-player-selection.md)で置き換えた。以下は初期実装の記録である。

## 背景

Init Saw以外の音色をtrackごとに選び、`pa`で別々の音色を同時演奏する。検証用のsaved SMFへ選択結果を保存し、次回の読込で復元する。0001の固定音色・単一音源という制限を置き換える。

## 決定

最初のブラウザはProgramData配下の`Surge XT/patches_factory`にある`.fxp`を再帰的に列挙する。シンボリックリンクは辿らず、カテゴリを含む相対パスで表示・検索する。同名パッチは相対パスで区別する。cat-plugin-playerのブラウザ全体の移植は行わず、既存のCLAP preset-loadを利用する。カテゴリ分類、ユーザーパッチ、複数プラグイン、GUI編集、state snapshotは今回の範囲に含めない。

Program欄またはProgram Change行からブラウザを開く。上下キー・クリック・検索結果の変更で現在tickのノートを試聴し、ノートがない場合はC4を0.5秒試聴する。試聴はSMFを変更しない。EnterまたはConfirmで確定し、Esc・Cancel・ウィンドウの閉じる操作は元の音色を保持する。読込に失敗したパッチはエラーを表示して確定しない。ブラウザの表示中は主画面の選択と演奏ショートカットを無効にする。

trackごとの音色識別を保持し、発音が必要になった時点で各trackのSurge XTインスタンスを生成する。インスタンスの組合せ・パッチが変わる場合は音声ストリームを停止して再生成する。同じ組合せは再利用する。一つのcpalストリームで全trackを同じフレーム時刻から演奏し、各trackの次の開始時刻・範囲末尾の最短境界で出力を分割して合成する。別trackの同pitchは独立した音源へ送り、Note Offが互いに干渉しない。同一track内の重複pitchは引き続きまとめる。合成後の出力を[-1,1]へ制限する。

CLAP bundleの初期化・終了は全インスタンスで共有する。UI threadで初期化し、最後のインスタンスが破棄されるまでLibraryを保持する。音源の生成・パッチ読込・main-thread callback・破棄はUI thread、processは一つのaudio callbackで行う。streamの破棄を全音源の破棄より先にする。依存ライブラリやcheckoutへの補修は導入しない。

## SMFへの記録

確定時に対象trackのtick 0へ次の3イベントを追加する。

- Device Name (`FF 09`): `Surge XT`
- Program Name (`FF 08`): パッチ名
- Sequencer Specific (`FF 7F`): 非商用ID `7D`、`cat-smf-editor\0instrument\0`という名前空間、JSON payload

JSONには`version = 1`、plugin ID、plugin名、patch名、factory rootからの相対パスを保持する。元のDevice/Program Nameとアプリ以外の独自メタを保存し、再確定では当アプリのブロックだけを置き換える。ノートの絶対tickを変更せずdeltaを再計算する。表示キャッシュと保存位置も再構築する。

アプリ用メタがないtrackはInit Sawを既定値とし、アプリ用メタの重複・未対応version/plugin・不正payloadは読込エラーにする。絶対パスや`..`を拒否し、実際のパッチはcanonicalizeしたfactory root内のファイルに限定する。指定パッチの欠損は演奏時にエラーとし、別音色へ黙って切り替えない。標準名イベントだけからパッチを推測しない。

確定した音色情報は既存のsaved SMF書出しを通じて保存する。同名savedを上書きしない検証用の運用は維持する。

## 検証と制限

検索・空検索結果、キーボード選択と確定・取消、試聴時の無変更、対象trackのみの変更、カーソル復元、SMF write/read、savedからの再読込、同名パッチの区別、欠損パッチと不正メタの拒否を自動テストする。全trackのフレーム境界と同pitchのduration分離も検証する。

インストール済みSurgeによるnativeレンダリングテストで、Init SawとDX EPの異なる出力、同pitchの別音源、片方の音源破棄後も他方が発音を続けることを確認する。多数trackや連続したパッチ変更では生成のCPU・メモリ負荷が増える。SMFのchannel、GM program、CC、pedal、途中の音色変更は引き続き再現しない。
