# ADR 0015: Vaporizer2のCustom modulator 1～4をMacro editの対象に加える

- 状態: 採用
- 関連: [0014](0014-surge-macro-edit.md)

## 決定

Vaporizer2 CLAP（`com.vastdynamics.VAST2`）のCustom modulator 1～4を、Surge XTのMacro 1～8と同じMacro edit・Variation・Macro variation curveの対象にする。保存meta・試聴・確定の経路は0014をそのまま使い、音源ごとに異なる「state内の使用判定・保存位置・名前」と「host parameterとの対応」だけを playerと共有する`cat-plugin-player/shared/instrument_macros.rs`（editorから `#[path]` で参照） に分ける。

## 音源固有の判定

- 使用判定：`VC2!`ラッパー内のXMLで、root直下の`m_uModMatSrceN`が`CustomModulatorK`のとき使用中とする。mod matrixに出ていないmacroは音に影響しないため、Surgeと同じく表示しない。
- 保存位置：`m_fCustomModulatorK`の0～100を0～1の操作位置にする。実機のCLAP parameter `Custom modulator K` は0～1のplainで、読み出し値はstateの値÷100と一致した。
- 名前：root属性`CustomModulatorKText`。既定の`CustomK`は無名として扱う（Surgeの`-`と同じ）。
- polarity：Vaporizer2のpolarityはmacroではなくmod matrixの行（`m_uModMatPolarityN`）ごとに持つ。macroが出ている行がすべてBipolarなら中央0の表示にし、Unipolarの行が1つでもあればmacro自身の0～1で表示する。表示だけの違いで、API送信値は常に0～1の操作位置である。Surgeと違い、bipolarでもAPI読み出し値は0～1のまま。

## 却下案

- player側の`shared/surge_macros.rs`にVaporizer2を足す案：Surge用のCC・mod wheel情報と混ざるため採らない。playerもVaporizer2のmacroをparameterで動かすようになったので、判定は`shared/instrument_macros.rs`でplayerと共有する（[player ADR 0039](../../../cat-plugin-player/docs/adr/0039-parameter-macro-sweep.md)）。Six Sines・Floeは[0016](0016-six-sines-floe-macro-edit.md)。
- polarityを一律unipolarにする案：Bipolarの行だけに出しているmacroでは中央が無効点になるのに、それが画面から読めなくなるため採らない。
