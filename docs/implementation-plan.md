# 鍼灸院予約ページ監視ツール (Rust + AWS Lambda + EventBridge + CDK) 実装プラン

## Context

人気鍼灸院の予約ページを定期的にチェックし、「空きあり」と判定されたタイミングで Email と Discord/Slack Webhook へ通知するツールを構築する。Rust と AWS の自己学習も兼ねるため、過度な抽象化を避けたシンプルな設計を優先する。

ユーザー要件の明確化結果:

- 監視対象は一般的な HTML レンダリングの予約ページ (JS 不要、reqwest + scraper で取得可能)
- 通知方式は **Email (AWS SES)** と **Discord / Slack Webhook** の併用
- 監視頻度は **10 分間隔** (EventBridge rate)
- **「空きあり」の判定ロジックはユーザーが後から実装する** — 本プランでは `fn has_availability(slots: &[Slot]) -> bool` のシグネチャと TODO 実装だけ用意する
- 通知制御は「フラグが true なら通知 / false なら何もしない」のみ。**状態管理 (S3 スナップショット, 差分検知) は実装しない**。スパム化が起きた場合はユーザー側で判定ロジックを絞り込んで対応する想定
- IaC は **AWS CDK (TypeScript)**
- デプロイ先想定リージョン: ap-northeast-1
- 想定月額コスト: $0.1 未満 (実質ゼロ、無料枠内)

## アーキテクチャ概要

```
EventBridge Rule (rate 10min)
        │
        ▼
   Lambda (Rust, arm64)
   ├─ reqwest で HTML 取得
   ├─ scraper で空き枠をパース → Vec<Slot>
   ├─ has_availability(&slots) で bool 判定 ← ユーザー実装
   └─ true の場合のみ並列通知
         ├─ SES SendEmail (自分宛)
         ├─ Discord Webhook POST
         └─ Slack Webhook POST

設定値:
  - Lambda 環境変数: TARGET_URL, EMAIL_FROM, EMAIL_TO, USER_AGENT
  - SSM Parameter Store (SecureString): /sozen-monitor/webhooks
    → 起動時に 1 回 GetParameter、JSON で discord / slack URL を保持
```

## リポジトリ構成

```
/Users/miwa/project/sozen-monitoring/
├── .gitignore                       # target/, node_modules/, cdk.out/, .env
├── README.md
├── rust-toolchain.toml              # channel = "1.82.0" 固定
│
├── lambda/                          # Rust Lambda 関数 (単一 crate)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                  # lambda_runtime エントリ + Deps 初期化
│   │   ├── handler.rs               # 取得 → 判定 → 通知 のフロー
│   │   ├── config.rs                # 環境変数 + SSM 読込
│   │   ├── scraper.rs               # fetch_html + parse_slots
│   │   ├── availability.rs          # has_availability(&[Slot]) -> bool ← ユーザー実装
│   │   ├── notifier/
│   │   │   ├── mod.rs
│   │   │   ├── email.rs             # SES v2 SendEmail
│   │   │   └── webhook.rs           # Discord / Slack POST
│   │   └── model.rs                 # Slot 構造体
│   └── tests/
│       ├── fixtures/sample_page.html
│       └── parse_test.rs
│
└── infrastructure/                  # AWS CDK (TypeScript)
    ├── package.json
    ├── tsconfig.json
    ├── cdk.json
    ├── bin/app.ts
    └── lib/monitoring-stack.ts
```

## Rust Lambda 関数

### 依存クレート (`lambda/Cargo.toml`)

```toml
[dependencies]
lambda_runtime    = "0.13"
tokio             = { version = "1", features = ["macros", "rt"] }
reqwest           = { version = "0.12", default-features = false, features = ["rustls-tls", "gzip"] }
scraper           = "0.20"
serde             = { version = "1", features = ["derive"] }
serde_json        = "1"
aws-config        = { version = "1", features = ["behavior-version-latest"] }
aws-sdk-sesv2     = "1"
aws-sdk-ssm       = "1"
tracing           = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
anyhow            = "1"
thiserror         = "1"

[profile.release]
lto           = "thin"
codegen-units = 1
strip         = true
opt-level     = "z"
```

`aws-sdk-s3` を含めない (状態管理なし)。`rustls-tls` で OpenSSL 不要、arm64 クロスビルドが楽。

### 主要シグネチャ

```rust
// model.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub date: String,             // "2026-05-25"
    pub time: String,             // "14:00"
    pub label: Option<String>,
}

// config.rs
pub struct Config {
    pub target_url: String,
    pub email_from: String,
    pub email_to: String,
    pub user_agent: String,
    pub discord_webhook_url: Option<String>,
    pub slack_webhook_url: Option<String>,
}
impl Config {
    pub async fn load(ssm: &aws_sdk_ssm::Client) -> anyhow::Result<Self>;
}

// scraper.rs
pub async fn fetch_html(client: &reqwest::Client, url: &str, ua: &str) -> anyhow::Result<String>;
pub fn parse_slots(html: &str) -> anyhow::Result<Vec<Slot>>;

// availability.rs (ユーザーが後から書く)
pub fn has_availability(slots: &[Slot]) -> bool {
    // TODO: 例えば「特定日時の枠が含まれるか」「総数が閾値以上か」など
    !slots.is_empty()
}

// notifier/email.rs
pub async fn send_email(ses: &aws_sdk_sesv2::Client, cfg: &Config, slots: &[Slot]) -> anyhow::Result<()>;

// notifier/webhook.rs
pub async fn post_discord(http: &reqwest::Client, url: &str, slots: &[Slot]) -> anyhow::Result<()>;
pub async fn post_slack(http: &reqwest::Client, url: &str, slots: &[Slot]) -> anyhow::Result<()>;

// handler.rs
pub async fn run(cfg: &Config, deps: &Deps) -> anyhow::Result<()> {
    let html = scraper::fetch_html(&deps.http, &cfg.target_url, &cfg.user_agent).await?;
    let slots = scraper::parse_slots(&html)?;
    if !availability::has_availability(&slots) {
        tracing::info!(slot_count = slots.len(), "no availability, skip notify");
        return Ok(());
    }
    let (e, d, s) = tokio::join!(
        notifier::email::send_email(&deps.ses, cfg, &slots),
        async { if let Some(u) = &cfg.discord_webhook_url { notifier::webhook::post_discord(&deps.http, u, &slots).await } else { Ok(()) } },
        async { if let Some(u) = &cfg.slack_webhook_url   { notifier::webhook::post_slack(&deps.http, u, &slots).await } else { Ok(()) } },
    );
    for r in [e, d, s] { if let Err(err) = r { tracing::error!(error = %err, "notify failed"); } }
    Ok(())
}

// main.rs
#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    init_tracing();
    let aws_cfg = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let ssm = aws_sdk_ssm::Client::new(&aws_cfg);
    let cfg = Config::load(&ssm).await?;
    let deps = Deps {
        http: reqwest::Client::builder().connect_timeout(Duration::from_secs(5)).timeout(Duration::from_secs(15)).build()?,
        ses: aws_sdk_sesv2::Client::new(&aws_cfg),
    };
    lambda_runtime::run(service_fn(|_e: LambdaEvent<serde_json::Value>| async {
        handler::run(&cfg, &deps).await.map_err(Into::into)
    })).await
}
```

### エラー方針

- 通知は **Email / Discord / Slack それぞれ独立に並列実行**、失敗は ERROR ログのみ。Lambda は Ok で終了 (重複通知を防ぐため EventBridge リトライ無効化と整合)。
- スクレイピング失敗は WARN ログを残して Ok 終了。パース 0 件は将来サイト改変の兆候として WARN を出す。

## CDK スタック (`infrastructure/lib/monitoring-stack.ts`)

主要 npm 依存:

```json
"dependencies": {
  "aws-cdk-lib": "^2.150.0",
  "constructs": "^10.3.0",
  "cargo-lambda-cdk": "^0.0.24"
}
```

リソース構成:

- **Lambda Function**: `RustFunction` (cargo-lambda-cdk)、arm64、memorySize 256、timeout 30s、`manifestPath` で `lambda/Cargo.toml` を指定
- **EventBridge Rule**: `Schedule.rate(Duration.minutes(10))`、`retryAttempts: 0`
- **CloudWatch Logs**: `RetentionDays.ONE_WEEK`
- **IAM ポリシー**:
  - `ses:SendEmail` (resources: verified identity ARN を指定)
  - `ssm:GetParameter` (resources: `/sozen-monitor/*`)
  - `kms:Decrypt` (condition: `kms:ViaService = ssm.<region>.amazonaws.com`)
- **環境変数**: `TARGET_URL`, `EMAIL_FROM`, `EMAIL_TO`, `USER_AGENT`, `WEBHOOK_PARAM_NAME=/sozen-monitor/webhooks`, `RUST_LOG=info`
- **S3 / DynamoDB は作らない** (状態管理なし)

CDK では作らず**手動コンソール操作**で済ませるもの:

- SES verified email identity 登録 (ap-northeast-1)
- SSM Parameter `/sozen-monitor/webhooks` (SecureString, JSON: `{"discord":"...","slack":"..."}`)

## デプロイ・検証手順

### 初期セットアップ

```bash
# Rust + cargo-lambda
rustup install stable
cargo install cargo-lambda

# CDK
npm install -g aws-cdk
aws configure   # or aws sso login

# SES verified identity (リージョン揃える)
aws ses verify-email-identity --email-address taiga.miwa@spectee.com --region ap-northeast-1
# → メール内リンクで承認

# SSM SecureString
aws ssm put-parameter \
  --name /sozen-monitor/webhooks \
  --type SecureString \
  --value '{"discord":"https://discord.com/api/webhooks/...","slack":"https://hooks.slack.com/..."}' \
  --region ap-northeast-1
```

### 開発フロー

```bash
cd /Users/miwa/project/sozen-monitoring
git init
cargo lambda new lambda --no-interactive --http=false
# infrastructure/ は npm init -y && npm install ... && cdk init app --language typescript を空ディレクトリで実行
```

### ローカル実行

```bash
cd lambda
AWS_PROFILE=<your-profile> cargo lambda watch
# 別シェルで:
cargo lambda invoke --data-ascii '{}'
```

### 単体テスト

サイトを `curl -A "<UA>"` で取得し `lambda/tests/fixtures/sample_page.html` に保存。`parse_test.rs` で `parse_slots` の回帰テストを書く。サイト構造変化時に最初に壊れる安全網になる。

### デプロイ

```bash
cd infrastructure
cdk bootstrap          # 初回のみ
cdk synth
cdk deploy             # 内部で cargo lambda build --release --arm64
```

### 動作確認

```bash
aws lambda invoke --function-name MonitorFn --payload '{}' /tmp/out.json
cat /tmp/out.json
aws logs tail /aws/lambda/MonitorFn --follow
```

EventBridge を一時的に `rate(1 minute)` に変えて 1〜2 サイクル試すと体験しやすい。検証後 10 分に戻す。

## コスト見積もり (ap-northeast-1)

- Lambda 実行: 4,320 invocations/月 × ~1s × 256MB = ~1,100 GB-s → **無料枠内 ($0)**
- Lambda リクエスト: 4,320/月 → **無料枠内 ($0)**
- EventBridge Rule (default bus): **$0**
- SES: 月数十通想定 → **$0** (Lambda 経由 62,000 通/月まで無料)
- CloudWatch Logs: ~50MB/月 → **$0.03 程度**
- SSM Parameter Store (Standard): **$0**
- KMS Decrypt: 4,320/月 → **無料枠内 ($0)**

**合計: 月 $0.1 未満**

## 注意点・リスク

### 法務・マナー

- **対象サイトの robots.txt / 利用規約を必ず確認**。Disallow / スクレイピング禁止条項がないことを確認してから運用開始
- **User-Agent に連絡先を含める**: 例 `sozen-monitor/0.1 (+contact: taiga.miwa@spectee.com)`。先方から見て透明性を確保
- 10 分間隔は許容範囲だが、対象サイトの規模が小さいなら 30 分〜1 時間に間引く判断も検討

### 技術的リスク

- **SES サンドボックス**: From / To 両方を verified identity に登録すれば自分宛送信は可。Lambda と同一リージョンに揃える
- **Lambda Rust の cold start**: arm64 + lto + strip で 150〜300ms 程度。10 分間隔だと毎回 cold だが許容範囲
- **HTML 構造変化**: パース結果 0 件のときに WARN ログを出し、将来 CloudWatch メトリクスフィルタ → SNS でアラート追加
- **タイムゾーン**: 予約時刻が JST 表示なら Slot にも JST 文字列で保持し、変換しない。バグ源を作らない
- **`has_availability` を実装し忘れると毎回通知が飛ぶ**: 初期値は `!slots.is_empty()` だが、実用前に必ずユーザー判定ロジックを書く

### 運用上の注意

- スパム化が起きた場合は `availability::has_availability` を絞り込む (例: 特定日時のみ、人数閾値超え時のみ)
- Webhook URL が漏れた場合は SSM Parameter を上書き + Discord / Slack 側で URL 再生成

## 設計を意図的にシンプルに保っている点

- 単一 crate (ワークスペース化しない)
- trait による DI を導入しない (モジュール関数を直接呼ぶ)
- 状態管理 (S3 / DynamoDB) なし
- 差分検知ロジックなし
- DLQ なし、Lambda Insights / X-Ray なし
- エラー型は `anyhow::Result` 中心、境界部のみ `thiserror`

Rust 学習の純粋な対象を「lambda_runtime / async / reqwest / serde / aws-sdk」に絞り込むことを優先している。

## Verification (実装後の確認手順)

1. `cd lambda && cargo build --release` がエラーなく通る
2. `cargo lambda watch` + `cargo lambda invoke --data-ascii '{}'` でローカル実行し、ログにパース件数が出る
3. `cargo test` で `parse_slots` のフィクスチャテストが通る
4. `cd infrastructure && cdk synth` がエラーなく通る
5. `cdk deploy` 後、`aws lambda invoke` で実環境動作確認、Email + Discord + Slack に通知が届く
6. EventBridge を一時 `rate(1 minute)` にして CloudWatch Logs を follow し、定期実行を目視確認 → 10 分に戻す
7. `has_availability` を `false` 固定にして通知が飛ばないことを確認 (フラグ駆動の動作検証)

## 実装着手前に決めたいこと (任意、デフォルト値で進行可)

- AWS リージョン (推奨: `ap-northeast-1`)
- CloudWatch Logs 保持期間 (推奨: 7 日)
- Lambda timeout (推奨: 30 秒)
- メモリ (推奨: 256 MB)
