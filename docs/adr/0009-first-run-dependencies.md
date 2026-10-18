# ADR 0009: 初回起動時の依存物導入

## 背景

editorだけをダウンロードした利用者が、playerと音源を別々に探す必要があり、音を出すまでに迷ってしまう。playerには既にSurge XTの初回導入があり、複製すると同意画面・winget引数・失敗処理が乖離する。

## 決定

- 起動時に既存のplayer探索順を使う。明示した`--player`が不正な場合はエラーとし、自動取得で置き換えない。
- 不在の場合だけplayer READMEのv0.1.0 Windows x64 ZIPを取得する。別スレッドでHTTPS取得し、180秒・256 MiBの上限を設ける。
- exeと同じディレクトリの一時領域に展開する。取得対象はルートのexeとLICENSEだけで、任意のZIPパスは展開しない。展開したexeの`--help`で`--resident`・`--restore-state`・`--patch-roots`を確認してからexeを公開し、既存exeは置換しない。一時領域は成功・失敗とも破棄する。
- MITの再同意画面は出さない。配布物の著作権表示をWindowsの既知のDesktopディレクトリ（`dirs::desktop_dir()`、移動済みDesktopにも対応）へ`cat-plugin-player-LICENSE.txt`として保存し、editorのLICENSEを保全する。同名で異なる内容があればSHA-256の先頭16桁を付けた別名で保存し、既存ファイルを上書きしない。Desktopへ保存できなければexeは配置せずエラーにする。
- 音源検出成功後、instrumentがなければSurge XTを案内する。effectのみも対象。検出失敗を音源不在として扱わない。
- playerの`shared/first_instrument_install.rs`を両アプリの正本とする。不在判定、Windows言語選択、GPLv3リンク、同意画面、wingetの実行・進捗・失敗表示、recording-mode用引数を共有する。アプリ固有の再スキャン・再生はadapterに残す。
- 通常起動では共通metadata readerで既存playerの`plugin-list-cache.json`を読み、実在するinstrumentの記録があればセットアップ走査を省略して従来のGUI前の再生を保つ。cacheがない・壊れている・音源の実体が消えている場合は検出する。cacheの形式と保存先は変更しない。
- editorはオーディオデバイスを開かずnative hostで検出する。導入後は強制再スキャンする。音色は従来のbrowserで選ぶ。
- 初回セットアップの後でMIDI復元・自動再生・ファイル選択を開始する。セットアップ中は通常画面のショートカットへ入力を渡さず、Tabによる項目移動を許可する。常設の入口を上部バーへ追加しない。

## 制約と保存

ネット接続・exeディレクトリの書込権限が必要。Surge XTは既存実装同様wingetを使い、必要ならUACが表示される。winget不在や失敗では手動取得先を示す。playerの自動更新は対象外。配布ZIPの版を更新する際はURLと実物のインストールテストを更新する。2026-10-18時点の配布ZIP（2026-10-15公開、player commit `62b84e4`）は連携機能の追加前なので配置前に拒否される。両repoの変更が公開mainへ反映され、双方の自動リリースが成功すれば解消する。両方とも毎日01:23 JSTに開始予定だが、完了順は連動していない。editorだけが先に公開された期間は、取得した古いplayerを拒否する。両方の公開完了後に取得する場合、この公開順の問題はない。

2026-10-18の運用確認では双方のscheduleはactiveだが、直近の[editor実行](https://github.com/cat2151/cat-smf-editor/actions/runs/39049706657)は`cmrt-core`の依存checkout不足、[player実行](https://github.com/cat2151/cat-plugin-player/actions/runs/39049653924)はreleaseテストの`STATUS_ACCESS_VIOLATION`で失敗し、公開はskipされている。定期実行の設定だけで更新成功を保証できないため、対応コードの公開mainへの反映と両実行の成功を確認する。

既存の曲・設定の保存形式は変更しない。LICENSEの旧配置先（workspaceと既定releaseディレクトリ）とDesktopの既存ファイルを確認したが、移動対象の旧`cat-plugin-player.LICENSE`は存在しなかったため一度限りの移行は不要。既存の曲、音色参照、設定、履歴の移行は不要。既存playerとeditorのLICENSEを上書きせず、Surge XTの再導入も既存音源がある場合は案内しない。

## 検証

共通側で音源不在判定・導入中の再スキャン・導入後のCLAP選択・winget引数をテストする。editor側でZIPの許可ファイル、壊れた配布物、既存exeの保全を検証し、実物ZIPを一時ディレクトリへ取得して`--help`を実行し、対応版の導入または未対応版の配置前拒否を確認するテストを明示実行する。実環境の音源をアンインストールして不在を再現しない。

DesktopへのLICENSE保存は、隔離先での同名・異内容の保全テストに加え、実際のDesktopへ`cat-plugin-player-LICENSE.txt`を保存してplayerのLICENSEとのバイト一致を確認した。
