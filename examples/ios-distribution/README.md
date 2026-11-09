# iPad クラウドビルドとインターネット配布の選択案

2026-11-06 JST 時点の提案です。第一候補は A の公開 GitHub standard macOS と、本人向け internal TestFlight です。実ビルドの RAM/ディスク適合、App Store Connect による Rust bundle の受理、外部回線からの実機更新は未検証です。採用判断者は Shinichi Ohno さんです。

## A/B の比較と採用条件

| 案 | 資源・版 | 費用の前提 | 採用条件と運用責任 |
| --- | --- | --- | --- |
| A: GitHub standard macOS ARM | M1 3 CPU、7 GB RAM、14 GB SSD。macos-26 の具体的 Xcode/SDK は各 run で記録します | 対象 fork は API で public を確認。standard compute は $0。artifact/cache は別枠です | cold build が容量内で成立し、待ち時間を含む実測が許容範囲なら採用します。fresh VM の cache 復元、image 更新の監視は利用者が担います |
| B: CodeBuild macOS reserved | us-east-1 M2 8 CPU/24 GiB/128 GB、または 12 CPU/32 GiB/256 GB。managed macos-arm-base:26 は Rust 1.92.0 / Xcode 26.2 | 24 GiB は $0.020/分、32 GiB は $0.036/分。最低 24 時間なので $28.80 / $51.84。4 回各24時間なら $115.20 / $207.36、30日常時なら $864 / $1,555.20。税・為替・S3・ログ・Secrets・KMS 等は別です | A の RAM/SSD が不足する場合、または AWS 統合が必須の場合の候補です。fleet、IAM、秘密情報、cache、終了・解放の管理を利用者が担います。ジョブ終了は fleet 解放ではありません |

A/B は排他的に選びます。`.example` は active workflow ではなく、コピーや project 作成まで実行されません。A は workflow_dispatch のみ、並列1本、unsigned build のみです。B も unsigned build のみで、fleet 作成・webhook・upload を含みません。署名 job は信頼する固定 revision と保護 environment に限定してから追加します。PR の任意コードに署名 secrets を渡しません。

B を採る場合は region、quota/空き、fleet 1台、上記 SKU、MAC_ARM 環境、managed image、service role、project の buildspec path をコンソールで確認し、見積と解放担当を決めてから作成します。最初は webhook と自動トリガーを無効にします。権限は checkout、対象 cache/artifact、対象 secret のみに限定し、復号を許す KMS も限定します。標準 image の toolchain をそのまま使わず、repo pin の Rust 1.96.1 と aarch64-apple-ios を追加します。Xcode 26.2 が不適合なら custom Mac AMI または EC2 Mac と再比較します。27.2 を最低要件と決めつけません。

EC2 Mac は CI サービスとは別の Dedicated Host です。AMI/runner/OS 更新と EBS cache を利用者が管理し、最低24時間課金があります。自由な Xcode 固定を優先する場合の代替です。Xcode Cloud は Rust artifact の archive 統合、Codemagic は cache/RAM、Buildkite/MacStadium は追加契約と永続環境の費用が選定条件です。今回はこれらの最新見積を再取得していないので、過去の単価を確定見積として転記しません。既存 Mac は追加契約不要ですが電源・回線・利用競合が残ります。

## 現行コードと確認できた範囲

基準 revision は `10489c57a8662e8ad2e0eabded2e58b0d93a78d4` です。

- `scripts/ios/bundle.sh:244` は Cargo --locked、`:275` 以降は app 組立、`:316` 以降は実機署名処理です。unsigned device bundle は exit 3 です。
- `scripts/ios/profile.py:183` 以降の entitlements 処理を再利用します。期限、Team、App ID を検査し、自アプリ用 Keychain group を導出します。
- `.github/workflows/ci.yml:36` などの upstream repository guard は fork の Rust checks が通った証拠になりません。
- `.github/workflows/ios.yml:53` は SheprdRemoteCore の Swift tests、`:58` は SheprdRemote.xcodeproj です。Rust GPUI 配布の代用にはできません。
- `ios/tools/testflight.sh:14` 付近は秘密情報なしで exit 0 にする設計です。本案では署名/upload の入力不足をエラーにします。
- `Cargo.toml:104` は UIKit git revision を固定しています。`docs/ios.md:355` のローカル patch は CI に持ち込みません。

ローカルの読み取り probe は Xcode 27.2 / Build 27B52 / iPhoneOS SDK 27.2 を返しました。Rust pin は rust-toolchain.toml の 1.96.1 です。AWS 26.2 との compile/link/bundle 互換性は未測定です。

標準のローカル profile 保存先2箇所では対象 Team の profile が3件あり、3件とも開発用でした。対象 bundle の exact App ID、prefix と Team の一致、自アプリの Keychain group の許可も確認しました。profile の端末情報や秘密情報は記録しません。これは Program 未加入の証明ではありません。GitHub API の repository environment と repository secret 名一覧は空でしたが、組織 secrets や Apple 側 provisioning の不存在は証明しません。Program の契約主体・有効期限・ASC app record・upload 権限は未確認です。

## 共通手順と失敗条件

Python 3.11 以上、選択した Xcode/SDK、rustup、pin の toolchain/target が必要です。既存 bundle/profile helper を使い、profile 自動作成や個人 login Keychain の unlock は行いません。

```sh
python3 scripts/ios/distribution.py plan
python3 scripts/ios/distribution.py cache-key --sdk 27.2 --xcode 27B52
# 専用の実行資源を確保した後だけ実行します。
CARGO_TARGET_DIR=/absolute/job-private/target python3 scripts/ios/distribution.py build
# build の最終行の app path を指定します。秘密情報は CI の private files として注入します。
python3 scripts/ios/distribution.py package --app /absolute/output/sheprd.app --output /absolute/output/build.ipa --build-number 1 --method testflight
python3 scripts/ios/distribution.py upload --ipa /absolute/output/build.ipa
```

package は IOS_P12_FILE、IOS_P12_PASSWORD、IOS_PROFILE_FILE を要求します。upload は ASC_KEY_FILE、ASC_KEY_ID、ASC_ISSUER_ID、ASC_APP_ID（ASC app record の数値 Apple ID）を要求します。必要なアカウントでは ASC_PROVIDER_PUBLIC_ID も指定します。署名用証明書/private key/profile と、ASC API key は別物です。最小権限と失効/更新手順も別々に管理します。API key だけでは .app を署名できません。証明書/profile の期限切れや identity 不一致は失敗にします。

package は一時専用 Keychain に import し、profile が許可した identity で署名します。作成コマンドの途中失敗も含め、通常終了・例外時は専用 Keychain の削除を試み、削除失敗時にも検索リストを復元します。cleanup が成功するまで IPA を公開せず、既存の出力を上書きしません。元 .app 内への出力、bundle 内の symlink、未対応の nested code は拒否し、version/profile の変更は専用コピーだけに行います。SIGKILL/ホスト停止時の cleanup は保証できないため、永続 worker は再利用前に残留 secret files と Keychain を確認し、secrets 注入元も always/finally で削除します。秘密情報をログ・cache・artifact に入れません。ネストした framework/dylib は署名方針を実装するまで拒否します。

bundle ID `com.shin1ohno.herdr-gpui`、Team `AL9567565U`、既存 prefix を保持します。同一 bundle ID だけでデータ保持を保証せず、署名 entitlements と実機上書き後の設定・SSH資格情報・接続を確認します。既存 app の削除は更新検証の代替にしません。

IPA を zip できたことと ASC 受理は別です。現行 Info.plist は開発 bundle 向けで、AppIcon/Assets.car、配布用 SDK メタデータ、privacy/encryption 宣言の受理は未検証です。`ITSAppUsesNonExemptEncryption=false` はテンプレートにありますが、SSH 等の機能を含めた輸出コンプライアンス判定を代替しません。ASC の validation/processing で拒否された項目を owning bundle helper に実装してから運用化します。

upload は `--upload-package <IPA> --api-key <ID> --api-issuer <UUID> --p8-file-path <path> --output-format json` を使います。provider は必要時のみ `--provider-public-id` で指定します。インストール済み Xcode 27.2 / altool 27.10.4 の `--help` と `--help upload-package` で確認しました。metadata は IPA 内の Info.plist から検査し、ASC_APP_ID はローカルの app record 前提条件として検査します。metadata 引数（`-t`、`--apple-id`、`--bundle-id`、`--bundle-version`、`--bundle-short-version-string`）はこの upload のヘルプにないため渡しません。API key は team issuer を持つ方式が対象です。実 upload の受理は未検証です。

認証不要のエラー経路比較用に `scripts/ios/probe_distribution_altool.py` がありますが、存在しない入力による失敗は全引数の受理証明にはなりません。この準備作業では実行していません。argv fixture は `python3 -m unittest discover -s scripts/ios -p 'test_distribution*.py'` で実行します。Apple transport・codesign・security の fixture 成功は実 upload 成功とは別です。

追加した既存 Mac / TestFlight / Ad Hoc の実行例と利用者の作業順は [PREPARE.md](PREPARE.md) を参照してください。各案は比較用であり、この作業で採用・登録・課金は行いません。

## 配布先と決定事項

本人向け internal TestFlight は ASC メンバーとしての権限と app record、処理完了、internal group への追加が必要です。group 自動配布と iPad の自動更新設定は別です。build の有効期間は90日です。external TestFlight は Beta App Review が加わります。処理・審査・更新時刻の固定保証はありません。

Ad Hoc OTA は distribution profile が登録端末を含むこと、署名 IPA、到達可能な HTTPS manifest と IPA URL が必要です。UDID は private に管理します。年間登録台数・証明書/profile の期限・配信アクセス制御を運用します。証明書や配信 URL を公開 repo に含めません。

Apple Developer Program は年99 USD、対応地域では現地通貨です。円額は購入画面で確認します。既存 Team の加入主体・契約状態を確認し、個人/法人・所有主体と必要権限を決めてから登録や課金を行います。

最小の意思決定は (1) A を最初に評価するか、AWS を必須とするか、(2) 既存 Team の所有主体/有効な Program・ASC 権限、(3) 本人 internal TestFlight を先行するか、Ad Hoc OTA を必要とするかです。受入完了は外部回線の iPad で新 build を取得し、上書き起動・設定/接続/資格情報保持を観測した時点です。

## 計測と cache 方針

同一 source/lock/toolchain/SDK/target/features/release profile を固定し、source SHA、runner image、SDK build、CPU/RAM、空き disk、Cargo jobs も記録します。A/B は各1ジョブで比較し、既存 target を共有しません。

1. cold: 新規 job-private target と Cargo download cache で実行します。復元なしの開始状態を記録します。
2. warm: 同じ source に対し download/target cache を復元して再実行します。復元秒数と cache hit を記録します。
3. 1ファイル変更: 専用 checkout の Rust production file を1本変更し、同じ cache から再実行します。変更 diff と base SHA を保存します。単なる touch は採用しません。

待ち行列、image/toolchain準備、cache復元、Cargo compile/link、bundle組立、sign/package、upload、ASC processing、審査、端末 update を別列に記録します。現行 build helper のタイマーは compile/link/bundle の合計です。分割は Cargo timings と bundle の時刻 probe を追加してから行い、合計値を compile 単体と表記しません。device update は実機観測時刻です。全ステージの実測値は現在未測定で、速度倍率は算出しません。

専用実行資源で `/usr/bin/time -l` と `cargo --timings` 相当の計測を行い、最大 RSS と compile/link の証跡を保存します。`du -sk` で job-private Cargo registry/git/target の容量を測り、圧縮サイズ、復元/保存秒数、最終空き disk を記録します。計測のための追加 compile と通常 build は混同しません。

cache key は toolchain/SDK/Xcode build/target/features/profile/lock hash を含みます。A の例は依存 download cache のみで compiled target は含みません。GitHub cache は repository 既定10GB・未アクセス7日削除が基準で、設定拡張や artifact は別の課金条件です。target が容量を圧迫する場合は保存せず download cache のみにします。B の local cache/同一 worker 再利用は保証せず、S3 等の復元経路・IAM・保持期限を実測後に設定します。sccache は incremental/link の制約を測ってから追加します。

## 一次資料

- [GitHub standard specs/free public compute](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [GitHub cache limits and eviction](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)
- [AWS runtime versions](https://docs.aws.amazon.com/codebuild/latest/userguide/available-runtimes.html)
- [AWS compute/RAM/disk](https://docs.aws.amazon.com/codebuild/latest/userguide/build-env-ref-compute-types.html)
- [AWS reserved fleets](https://docs.aws.amazon.com/codebuild/latest/userguide/fleets.html)
- [AWS pricing and minimum24h](https://aws.amazon.com/codebuild/pricing/)
- [AWS primary price JSON](https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/CodeBuild/current/index.json) — publicationDate 2026-09-11T12:46:25Z、us-east-1 の reserved.arm.m2.medium/large を2026-11-06に取得しました。
- [Apple Program enrollment](https://developer.apple.com/jp/programs/enroll/)
- [Internal TestFlight](https://developer.apple.com/help/app-store-connect/test-a-beta-version/add-internal-testers/)
- [TestFlight overview](https://developer.apple.com/help/app-store-connect/test-a-beta-version/testflight-overview/)
- [App ID prefix/Keychain](https://developer.apple.com/library/archive/qa/qa1726/_index.html)
- [Ad Hoc OTA manifest](https://help.apple.com/xcode/mac/current/en.lproj/devde46df08a.html)

- [Apple altool command/metadata reference](https://help.apple.com/asc/appsaltool/en.lproj/static.html)
- [Apple upload builds / processing boundary](https://developer.apple.com/help/app-store-connect/manage-builds/upload-builds)
