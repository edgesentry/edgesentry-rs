# ビルドとリリース

## リリース成果物のビルド

```bash
cargo build --workspace --release
```

特定のクレートのみをビルド：

```bash
cargo build -p edgesentry-rs --release
```

## crates.io への公開

1) まず品質ゲートを検証：

```bash
./scripts/run_unit_and_license_check.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

2) 一度だけログイン：

```bash
cargo login <CRATES_IO_TOKEN>
```

3) ドライラン公開：

```bash
cargo publish --dry-run -p edgesentry-rs
```

4) 公開：

```bash
cargo publish -p edgesentry-rs
```

## GitHub Actions リリース自動化（ macOS / Windows / Linux ）

このリポジトリには`.github/workflows/release.yml`が含まれています。

- トリガー：`v0.1.0`のようなタグをプッシュ
- 品質ゲート：ビルド・ユニットテスト・ライセンスチェック・ clippy
- `edgesentry-rs`を crates.io に公開
- Linux ・ macOS （ x64 + arm64 ）・ Windows 向けの`eds`バイナリをビルド
- パッケージ済みバイナリを GitHub Release アセットにアップロード

注意：`.github/workflows/ci.yml`は`edgesentry-rs`の`cargo publish --dry-run`を実行します。

必要な GitHub シークレット：

- `CRATES_IO_TOKEN`：`cargo publish`が使用する crates.io API トークン

## マージ後の自動バージョンインクリメント

このリポジトリには`.github/workflows/auto-version-tag.yml`も含まれています。

- トリガー：`main`で CI が成功したとき
- アクション：`Cargo.toml`の`workspace.package.version`を更新し、`vX.Y.Z`タグを作成・プッシュ
- その後：そのタグによって`release.yml`がトリガーされ、完全なリリースパイプラインが実行される

バージョンバンプルール（ Conventional Commits ）：

- `fix:` -> パッチバンプ（`x.y.z` -> `x.y.(z+1)`）
- `feat:` -> マイナーバンプ（`x.y.z` -> `x.(y+1).0`）
- `!`または`BREAKING CHANGE` -> メジャーバンプ（`x.y.z` -> `(x+1).0.0`）
