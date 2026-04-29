# EdgeSentry Inspect への貢献

## 整合性チェック

コード・テスト・スクリプト・ドキュメントのいずれかを変更するたびに、3 つの層が同期していることを確認してください。

1. **コード → ドキュメント：** モジュール・関数・CLI コマンド・動作を追加・削除・名前変更した場合は、それを参照するすべてのドキュメントを更新してください（`architecture.md`・`cli.md`・`demo.md`・`roadmap.md`）。
2. **ドキュメント → コード：** ドキュメントが機能やコマンドを説明している場合は、それが存在し、説明通りに動作することを確認してください。古くなったサンプルや誤った Cargo フィーチャー名は CI 失敗の原因になります。
3. **スクリプト → コード：** テストファイルや Cargo フィーチャーの名前を変更した場合は、それを参照するすべてのスクリプトとワークフローを更新してください（例：`.github/workflows/ci.yml`）。

PR 作成前の簡単な grep チェック：

```bash
# 変更したシンボルに言及しているドキュメントを検索
grep -r "<old-name>" docs/ scripts/ .github/
```

---

## クレート構成

| クレート | 役割 |
|---------|------|
| `edgesentry-inspect` | IFC ローダー・偏差エンジン・ヒートマップレンダラー・JSON レポート |
| `eds` | 統合 CLI バイナリ — `eds inspect scan` エントリーポイント |
| `trilink-core` | 点群の投影・逆投影（上流依存） |

---

## Issue ラベル

すべての Issue には 1 つの **タイプ** ラベル、1 つの **優先度** ラベル、1 つ以上の **カテゴリ** ラベルを付けてください。

### タイプラベル

| ラベル | 使用場面 |
|-------|---------|
| `bug` | 何かが壊れているか、誤った動作をしている |
| `enhancement` | 新機能または既存動作の改善 |
| `documentation` | ドキュメントのみの変更 — プロダクションコードへの影響なし |

### 優先度ラベル

| ラベル | 意味 | 例 |
|-------|------|----|
| `priority:P0` | 必須 — リリースまたはコアパイプライン機能をブロックする | IFC ローダーの破損、偏差エンジンのパニック、有効な入力での CLI クラッシュ |
| `priority:P1` | あると良い — 高い価値があり、近い将来に予定されている | 組み込み推論モデル、デモウォークスルー、ビジュアライゼーションプロトタイプ |
| `priority:P2` | 良ければ持ちたい — 価値はあるが後回しにできる | コンプライアンスレポート生成、パートナーセンサープラグイン |
| `priority:P3` | 低優先度 — 緊急性のない改善 | CI 最適化、マイナーな DX 改善 |

判断に迷う場合は「これはユーザーが `eds inspect scan` をエンドツーエンドで実行することをブロックするか？」と問いかけてください。Yes なら P0。体験を大幅に改善するなら P1。後のマイルストーンで提供できるなら P2。

### カテゴリラベル

| ラベル | 使用場面 |
|-------|---------|
| `core` | 偏差エンジン・IFC ジオメトリ・ヒートマップ・レポートシリアライゼーション |
| `compliance-governance` | CONQUAS / MLIT レポート生成、ISO 19650 統合 |
| `devsecops` | CI/CD パイプライン・静的解析・リリース自動化 |
| `platform-operations` | フィールド PC デプロイメント・クラウド同期・インフラ |
| `hardware-needed` | 物理 LiDAR / ToF センターハードウェアが必要（常に `priority:P2` と組み合わせること） |

---

## プルリクエストの規約

常にブランチを作成したユーザーをアサインしてください：

```bash
gh pr create --assignee "@me" --title "..." --body "..."
```

---

## 必須：コード変更後は必ずテストを実行する

**毎回** のコード変更後に実行：

```bash
cargo test --workspace
```

すべてのテストが通過するまで変更が完了したとみなさないでください。

---

## テストの実行

### 前提条件（macOS）

```bash
brew install rustup-init
rustup-init -y
source "$HOME/.cargo/env"
rustup default stable
```

### ユニットテスト

```bash
# 全クレート
cargo test --workspace

# Inspect クレートのみ
cargo test -p edgesentry-inspect
```

### 統合テスト（CLI エンドツーエンド）

```bash
cargo test -p eds --features transport-http,transport-tls --test cli_integration
```

---

## 静的解析とライセンスチェック

PR を開く前に実行：

```bash
# リント
cargo clippy --workspace --all-targets --all-features -- -D warnings

# セキュリティアドバイザリ
cargo audit

# OSS ライセンスポリシー
cargo deny check licenses
```

---

## main とのコンフリクトを避ける

**作業開始前に：**

```bash
git fetch origin
git checkout main && git pull origin main
git checkout -b <your-branch>
```

**ブランチを最新の状態に保つ** — PR を開く前に main にリベース：

```bash
git fetch origin
git rebase origin/main
```

**最もコンフリクトしやすいファイル — 編集前に調整してください：**

| ファイル | 頻繁にコンフリクトする理由 |
|---------|------------------------|
| `docs/inspect/en/src/demo.md` | 複数の PR がデモウォークスルーを拡張する |
| `docs/inspect/en/src/cli.md` | CLI フラグやサブコマンドが変わるたびに更新される |
| `docs/inspect/en/src/roadmap.md` | 作業完了に伴いマイルストーンのステータスが更新される |
| `.github/workflows/ci.yml` | フィーチャーと CI 改善の両方の PR で触れられる |
