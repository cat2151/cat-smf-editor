# ADR 0022: trackごとのplugin処理をworker threadで並列にする

- 状態: 採用
- 決定者: user（音源を多く使う曲でも再生中に音が途切れないこと）。並列化という直し方、worker数、threadの優先度、待ち方はAIの設計
- 関連: [0001](0001-surge-audition.md)、[0002](0002-track-patches.md)、[0007](0007-effect-chain-on-uapmd-graph.md)

## 決定

1つのdevice blockの中で、part（1 trackの楽器とtrack effect）どうしをworker threadで並列に処理する。0001と0002の「processは一つのaudio callbackで行う」はこのADRで置き換える。

- 同じpartの楽器とtrack effectは、1つのthreadで順に処理する。並列にするのはpartどうしだけ。
- 全partは同じ時刻から同じframe数を鳴らす。各partは自分のnote・timeline・automationの境界だけで区切る。
- mix・master effect・auto gain・clickは、全partの完了後にpart番号順に直列で行う。出力は直列処理とbit一致する。
- stream（曲・Virtual MIDI keyboard・offlineの書き出し）ごとに別のworker poolを持つ。
- 参加するthreadは、part数と論理CPU数の半分の小さい方。audio callbackのthreadも参加者の1人。
- 実時間のstreamでは、worker threadとcallbackのthreadをTIME_CRITICALにする。offlineの書き出しは上げない。
- workerの1本は次のblockをspinで待ち、残りはparkで待つ。全workerが周期の大半を休むと、pluginの処理が約2.5倍遅くなった（全core共通のclockが下がるためとみる）。1本が回り続けると遅くならない。代わりに鳴っているstreamごとに1 core分のCPUを使う。
- 鳴っていないstreamでは、spinもplugin・effect・master effectの処理も止める。曲の終わり・停止の後や、keyboardを弾いていない間も全patchを処理し、何も鳴らしていない本体が約2 core分を使い続けたため。「鳴っていない」は、再生中の曲・押されたままの音程や鍵・keyboardの未処理の入力・phrase loop・metronomeのどれも無く、最終出力が-90 dBFS未満で1秒続いたこと。余韻やeffectの尾は出力が続く間は切らない。止めている間も再生位置は進める。
- worker内のpanicは、そのpartを無音にして失敗を立てる。callbackは待ち続けない。

## 別threadからpluginを処理してよい根拠

- pluginが問い合わせる「audio threadか」の答えは、remidyのthread_localの旗で、shimがinstrumentとeffectのprocessの中で立てる。どのthreadからprocessを呼んでもpluginにはaudio threadに見える。
- 1個のprocessorは自分のcontextとscratchを持ち、processの経路はhost全体の状態に触らない。
- uapmd自身のengineも、trackのplugin graphをworker poolで並列に処理している。

## 却下した案

- 曲の再生中はVirtual MIDI keyboardのstreamを止める: 本体のGUIが無い書き出し・probeでも周期に間に合っていないので、これだけでは足りない。
- callbackとは別threadで先に音を作り、ring bufferに貯める: 1 blockの平均が周期に届いている負荷では効かない。
- MMCSS（Pro Audio）への登録と、power throttlingの解除でclockを保つ: 実測でpluginの遅さが変わらなかった。

## 変えないこと

- pluginの生成・破棄・patch読込・main-thread callbackはUI threadのまま。
- crateを足さない。Windows APIは`kernel32`の宣言で呼ぶ。
- `../cat-plugin-player`とuapmdは変えない。
