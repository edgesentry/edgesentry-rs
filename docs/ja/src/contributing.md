# コントリビューション

## 整合性チェック

コード・テスト・スクリプト・ドキュメントのいずれかを変更するたびに、 3 つの層が同期していることを確認してください。

1.**コード→ドキュメント：**モジュール・関数・ CLI コマンド・動作を追加・削除・名前変更した場合は、それを参照するすべてのドキュメントを更新してください（`concepts.md`・`architecture.md`・`cli.md`・`quickstart.md`・`demo.md`・`traceability.md`）。
2.**ドキュメント→コード：**ドキュメントが機能やコマンドを説明している場合は、それが存在し、説明通りに動作することを確認してください。古くなったサンプルや誤ったテストターゲット名は CI 失敗の原因になります。
3.**スクリプト→コード：**テストファイルや Cargo フィーチャーの名前を変更した場合は、それを参照するすべてのスクリプトとワークフローを更新してください（例：`scripts/integration_test.sh`・`.github/workflows/`）。
4.**トレーサビリティ：**コンプライアンスコントロールを実装または変更した場合は、`docs/src/traceability.md`のステータスを更新してください（✅ / ⚠️ / 🔲）。

PR 作成前の簡単な grep チェック：

```bash
# Find docs that mention a symbol you changed
grep -r "<old-name>" docs/ scripts/ .github/
```

---

## Issue ラベル

すべての Issue には 1 つの**タイプ**ラベル、 1 つの**優先度**ラベル、 1 つ以上の**カテゴリ**ラベルを付けてください。

### タイプラベル

| ラベル | 使用場面 |
|-------|-------------|
| `bug` | 何かが壊れているか、誤った動作をしている |
| `enhancement` | 新機能または既存動作の改善 |
| `documentation` | ドキュメントのみの変更 — プロダクションコードへの影響なし |

### 優先度ラベル

| ラベル | 意味 | 例 |
|-------|---------|---------|
| `priority:P0` | 必須 — 対象標準（ CLS ・ JC-STAR ・ CRA ）を満たすために直接必要。解決するまで作業はブロックされる | 壊れた署名検証、欠けたハッシュチェーンリンク、失敗しているインテグリティゲート |
| `priority:P1` | 望ましい — コンプライアンスの態勢や開発者体験を強化するが、標準適合のハードブロッカーではない | 鍵ローテーションツール、 CI 強化、トレーサビリティマトリックス、 FFI ブリッジ |
| `priority:P2` | ベストエフォート — ストレッチゴール・あると良いもの・専用ハードウェアを必要とするもの。余裕があれば取り組む | HSM 統合、教育白書、リファレンスアーキテクチャ |

判断に迷う場合は「標準が明示的にこれを要求しているか？」と問いかけてください。 Yes なら P0 。標準で義務付けられていないが役立つなら P1 。ストレッチゴール・望ましい追加・ハードウェア依存の作業なら P2 。

### カテゴリラベル

| ラベル | 使用場面 |
|-------|-------------|
| `core` | コアセキュリティコントロール — 署名・ハッシュ・インテグリティゲート・インジェストパイプライン |
| `compliance-governance` | コンプライアンスエビデンス・トレーサビリティマトリックス・開示プロセス |
| `devsecops` | CI/CD パイプライン・サプライチェーンセキュリティ・静的解析・監査ツール |
| `platform-operations` | インフラ・デプロイメント・運用準備 |
| `hardware-needed` | 物理ハードウェアまたはハードウェアバックのインフラが必要（常に`priority:P2`と組み合わせること） |

---

## プルリクエストの規約

プルリクエストを作成する際は、常にブランチを作成したユーザーをアサインしてください。

```bash
gh pr create --assignee "@me" --title "..." --body "..."
```

## 必須：コード変更後は必ずテストを実行する

**毎回**のコード変更後に実行：

```bash
cargo test --workspace
```

すべてのテストが通過するまで変更が完了したとみなさないでください。

## ユニットテスト

### 前提条件（ macOS ）

まず Rust ツールチェーンをインストール：

```bash
brew install rustup-init
rustup-init -y
source "$HOME/.cargo/env"
rustup default stable
```

`cargo-deny`をインストール（ OSS ライセンスチェックに必要）：

```bash
cargo install cargo-deny
source "$HOME/.cargo/env"
cargo deny --version
```

### テストの実行

すべてのユニットテストを実行：

```bash
cargo test --workspace
```

特定のクレートのテストを実行：

```bash
cargo test -p edgesentry-rs
```

S3 互換バックエンドフィーチャーを有効にして`edgesentry-rs`クレートを実行：

```bash
cargo test -p edgesentry-rs --features s3
```

ライブ MinIO インスタンスに対して S3 統合テストを実行（以下の環境変数が設定されている必要があります）：

```bash
TEST_S3_ENDPOINT=http://localhost:9000 \
TEST_S3_ACCESS_KEY=minioadmin \
TEST_S3_SECRET_KEY=minioadmin \
TEST_S3_BUCKET=bucket \
cargo test -p edgesentry-rs --features s3 --test integration -- --nocapture
```

4 つの`TEST_S3_*`変数のいずれかが未設定の場合、テストは自動的にスキップされます。

ユニットテストと OSS ライセンスチェックを 1 コマンドで実行：

```bash
./scripts/run_unit_and_license_check.sh
```

## 静的解析と OSS ライセンスチェック

リリース前に以下のチェックを使用してください。

### 1) 静的解析（`clippy`）

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### 2) 依存関係セキュリティアドバイザリチェック（`cargo-audit`）

一度だけインストール：

```bash
cargo install cargo-audit
```

実行：

```bash
cargo audit
```

### 3) 商用利用 OSS ライセンスチェック（`cargo-deny`）

一度だけインストール：

```bash
cargo install cargo-deny
```

ライセンスチェックを実行（ポリシーは`deny.toml`に定義）：

```bash
cargo deny check licenses
```

完全な依存関係ポリシーチェック（オプション）：

```bash
cargo deny check advisories bans licenses sources
```

このチェックが失敗した場合は、違反しているクレートを調べ、法務/セキュリティレビューの後にのみ依存関係またはポリシーを更新してください。

---

## main とのコンフリクトを避ける

コンフリクトは、フィーチャーブランチが main から分岐した後、同じファイルに触れる他の PR が main にマージされると発生します。このリポジトリで最もコンフリクトしやすいファイルは`scripts/local_demo.sh`・`docs/src/demo.md`・`.github/copilot-instructions.md`です。

**作業開始前に**

```bash
git fetch origin
git checkout main && git pull origin main
git checkout -b <your-branch>
```

**ブランチを最新の状態に保つ**— 特に PR を開く前に、定期的に main にリベースしてください：

```bash
git fetch origin
git rebase origin/main
```

**リベース中のコンフリクトの解消**

1. コンフリクトしているファイルを特定：`git diff --name-only --diff-filter=U`
2. 各ファイルについて、どちら側を保持するかを決定：
   -**自分のバージョンを採用：** `git checkout --theirs <file>`
   -**main のバージョンを採用：** `git checkout --ours <file>`
   -**手動でマージ：** `<<<<<<<` / `=======` / `>>>>>>>`マーカーを削除するようにファイルを編集する
3. 解消したファイルをステージ：`git add <file>`
4. 続行：`GIT_EDITOR=true git rebase --continue`
5. 次のコミットで再度コンフリクトが発生した場合は、ステップ 1 から繰り返す。

**解消後、リベースしたブランチを force-push ：**

```bash
git push --force-with-lease origin <your-branch>
```

**最もコンフリクトしやすいファイル — 編集前に調整してください：**

| ファイル | 頻繁にコンフリクトする理由 |
|------|----------------------|
| `scripts/local_demo.sh` | 複数の PR がステップを追加したりデモフローを再構成したりする |
| `docs/src/demo.md` | デモスクリプトの変更を反映する |
| `.github/copilot-instructions.md` | 新しいモジュールやサンプルが追加されるたびに構造セクションが更新される |
| `crates/edgesentry-rs/examples/lift_inspection_flow.rs` | クイックスタートの改善とロール境界の両方の作業で触れられる |
