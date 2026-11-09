# iPad ビルド・配布の実行準備

例は採用前の比較用です。`.example` は GitHub / launchd に登録されていません。署名・upload は `IOS_DISTRIBUTION_EXECUTE=true` の明示指定まで無効です。plan の既定は `execution=false` です。Cargo gate と実サービス検証が完了するまで push / PR / 本番運用は行いません。

## 比較できる5つの例

料金は2026-11-05調査の USD・税別、月800分（40回×20分）という比較仮定です。速度の予測ではありません。元資料は `/tmp/herdr-build-distribution-research.zEDoWT/report.md` の「同じ実行時間を仮定した月額例」です。実行前に利用者が最新料金を確認します。

| 選択肢と例 | 前提・既定動作 | 月額の比較値 |
| --- | --- | --- |
| GitHub hosted: `github.yml.example` | 公開repo・Actions利用権限、macOS/Xcode・Python3.11+・rustup。配置後も手動 dispatch のみ、unsigned | standard public $0。private gross $49.60、含有枠・保存料金は別です |
| CodeBuild: `codebuild.yml.example` | 利用者が AWS reserved macOS fleet / project / IAM / image / region / quota / 請求を準備。unsigned、webhookなし | M2 24GiB $0.020/分。週1回24h×4で $115.20、毎日20日確保 $576、730h常時 $876。32GiBは週1回 $207.36。800分だけ課金される構成ではありません |
| 既存Mac: `self-hosted.yml.example`、`self-hosted.md`、`runner.launchd.plist.example` | 利用者が専用ユーザー・runner登録・launchd設定。手動 dispatch、unsigned、専用1枠。Xcode等が必要です | GitHub compute $0、電力・機材・回線・保守別です |
| TestFlight internal: `testflight.sh.example` | Program・ASC app record・配布証明書/private key・App Store profile・ASC API key。既定は plan のみ | Program 年$99（単純月割$8.25）。選んだビルド経路・保存料金は別です |
| Ad Hoc OTA: `adhoc.sh.example`、`scripts/ios/ota.py` | 実配布は Program・端末UDID・Ad Hoc profile・証明書/private key・HTTPS配信先。既定はfixtureのローカルページ生成だけです | Program 年$99（単純月割$8.25、TestFlightと共用）。HTTPS配信の料金は構成未選定のため未見積です |

Apple Program の円額は購入画面で確認します。既存契約が有効なら追加加入しません。TestFlight と OTA はビルド経路に組み合わせる配布案です。二つの build 例を同時常用する前提ではありません。Codemagic / Xcode Cloud / EC2 / 専用ホステッドMacは元調査の追加候補で、この5例の対象外です。

## 利用者が行う順序

1. まず専用のCPU・ディスク枠を確保します。後続担当が fmt → build → test → strict clippy → exact-head `just ci` → strict `just lint-ios` を通し、署名コミットを確認してから push / PR を行います。今回 Cargo は実行しません。
2. 無署名で GitHub hosted または既存Macを試します。GitHub は例を workflow に配置して手動実行、既存Macは `self-hosted.md` の runner 登録から始めます。CodeBuild を比較する場合だけ、利用者が AWS 資源と課金・解放責任を設定します。自動作成コマンドはありません。
3. 各候補で同じrevisionの cold / warm / 1ファイル変更を順に計測し、待ち時間、cache復元、compile/link/bundle、RSS、空きディスクを記録します。既存targetと共有しません。GitHub standardの7GB RAM / 14GB SSD適合、Xcode26.2と27.2の互換性は未検証です。
4. 配布試験に進む場合、利用者が既存 Team の Program 有効性・加入主体を確認し、必要な場合のみ加入・年額支払いを行います。既存 bundle / Team / prefix / Keychain group を維持します。
5. TestFlight は利用者が ASC app record（bundle ID対応）、数値 Apple ID、権限、internal group を準備します。配布証明書/private key・App Store profile・team issuer API keyを作成し、CI専用保管先と権限・更新/失効責任を決めます。`IOS_P12_FILE` / `IOS_P12_PASSWORD` / `IOS_PROFILE_FILE` / `ASC_KEY_FILE` / `ASC_KEY_ID` / `ASC_ISSUER_ID` / `ASC_APP_ID`（必要時 `ASC_PROVIDER_PUBLIC_ID`）を private に注入します。API key は署名鍵の代用ではありません。
6. 信頼するrevisionと保護された実行枠で `IOS_APP` / `IOS_IPA`（新規出力）/ `IOS_BUILD_NUMBER` を指定し、`IOS_DISTRIBUTION_EXECUTE=true bash examples/ios-distribution/testflight.sh.example` を実行します。ASC validation / processing、必要な AppIcon・SDK metadata・privacy/encryption 宣言を確認し、internal group 配布後に iPad で更新・起動を確認します。
7. Ad Hoc も比較する場合、利用者が端末UDIDを登録し専用profileを発行します。上記署名変数と `IOS_APP` / `IOS_IPA` / `IOS_BUILD_NUMBER` / `OTA_OUTPUT` / `OTA_BASE_URL` を指定して `IOS_DISTRIBUTION_EXECUTE=true bash examples/ios-distribution/adhoc.sh.example` を実行します。生成された3ファイルを到達可能・アクセス制御された HTTPS 配信先へ利用者が配置します。この例は配信資源を作成しません。期限・端末台数・配信費用を管理します。
8. 外部回線の iPad で新buildの上書き起動と設定・SSH資格情報・接続の保持を確認します。upload exit 0 やmanifest生成だけで完了扱いにしません。実験後に AWS fleet解放・runner停止・配信削除の状態を確認します。

## 最初の実験

Cargo gate 通過後、公開 GitHub hosted の unsigned 手動1回を推奨します。Appleの加入・鍵作成を待たず、RAM/ディスクとビルド時間を観測できます。比較用に既存Macも1枠で実行します。CodeBuild は最低24h課金を利用者が承認してからです。

OTA の構造だけなら、Cargo・HTTPS・署名・アカウントなしで次を実行できます。fixture は実機にインストールできません。

```sh
python3 -m unittest discover -s scripts/ios -p 'test_ota.py'
# fixture IPA を自分で用意した場合。既定はリンク無効、ローカル生成だけです。
IOS_IPA=/absolute/fixture.ipa OTA_OUTPUT=/absolute/fresh-preview bash examples/ios-distribution/adhoc.sh.example
# fresh-preview/index.html をブラウザで開きます。ローカルHTTP/HTTPSサーバーは不要です。
```

準備済みは argv / profile / package / OTA fixture と構文検査までです。実署名、ASC受理、HTTPS OTA配信、端末インストール、データ保持、cloud実ビルドは未検証です。
