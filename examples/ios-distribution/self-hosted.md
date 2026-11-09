# 既存 Mac runner の試験

費用は月800分でも GitHub compute $0、電力・機材・保守は別です（2026-11-05 調査）。Apple 契約は unsigned 試験には不要です。Xcode、Python 3.11以上、rustup、十分な専用ディスク領域と専用ログインユーザーが前提です。8GiB Mac の他ビルドと同時実行しません。

利用者が次の順で設定します。この資料は登録・インストールを行いません。

1. リポジトリ Settings → Actions → Runners の macOS ARM64 手順で runner を専用ディレクトリへ取得し、表示されるハッシュを照合します。実行バージョンを固定せず、画面に表示された配布物を使います。
2. 同じ画面の一時登録トークンで `config.sh` を実行し、`herdr-ios` ラベルを付けます。トークンはファイル・履歴・報告に保存しません。信頼するコード専用とし、外部 PR は実行しません。
3. `runner.launchd.plist.example` の絶対パスを置換します。専用ユーザーで Xcode 初期設定・ライセンスを済ませ、Python/rustup が PATH 上にあることを確認します。
4. plist を専用ユーザーの `~/Library/LaunchAgents/` にコピーし、`plutil -lint <plist>` 後、`launchctl bootstrap gui/$(id -u) <plist>` で登録します。runner 公式 `svc.sh` を使う場合、この plist と二重登録しません。
5. GitHub の runner 表示が Idle になったことを確認してから `self-hosted.yml.example` を `.github/workflows/` に採用し、固定した信頼する branch の手動実行を1回行います。sleep・電源・回線断への運用を決めます。inbound ポート開放は不要です。
6. 停止時は `launchctl bootout gui/$(id -u) <plist>`、runner 登録解除は Settings の指示に従います。ログと専用 checkout に秘密情報を残しません。

テンプレートは例ディレクトリにある限り無効です。署名/upload は含めず、workflow_dispatch だけで unsigned build を実行します。専用ユーザーの login session がない場合の launchd 起動は未検証です。
