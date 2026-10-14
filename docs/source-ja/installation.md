---
source_sha256: 58c223c1500a
---

# インストール

`optopus` はコンパイル済みの拡張モジュールです (Rust + [PyO3](https://pyo3.rs)、
[maturin](https://www.maturin.rs) でビルド)。主要なプラットフォーム向けにビルド済みの wheel を
公開しているので、Rust ツールチェーンが必要なのはソースからビルドするときだけです。

(install-a-prebuilt-wheel-no-rust-required)=
## ビルド済み wheel のインストール (Rust 不要)

wheel は各 [GitHub リリース](https://github.com/trash-iine/optopus-py/releases)に添付されています。
プラットフォームに合うものをインストールしてください。

```bash
# Linux x86_64
$ pip install https://github.com/trash-iine/optopus-py/releases/download/v0.1.0/optopus-0.1.0-cp39-abi3-manylinux_2_17_x86_64.manylinux2014_x86_64.whl

# macOS (Apple Silicon)
$ pip install https://github.com/trash-iine/optopus-py/releases/download/v0.1.0/optopus-0.1.0-cp39-abi3-macosx_11_0_arm64.whl

# macOS (Intel)
$ pip install https://github.com/trash-iine/optopus-py/releases/download/v0.1.0/optopus-0.1.0-cp39-abi3-macosx_11_0_x86_64.whl
```

別のバージョンを入れるには、`v0.1.0` とファイル名中のバージョンを目的のタグに置き換えます。拡張は
安定 ABI (abi3) を対象にしているので、1 プラットフォームにつき 1 つの wheel で CPython 3.9 以降を
すべてカバーします。Python のマイナーバージョンを合わせる必要はありません。

(install-from-source)=
## ソースからのインストール

ビルド済み wheel のないプラットフォーム (Windows、aarch64 Linux、musl) では、リポジトリから
インストールします。

```bash
$ pip install git+https://github.com/trash-iine/optopus-py.git
```

特定のタグやコミットに固定するには次のようにします。

```bash
$ pip install git+https://github.com/trash-iine/optopus-py.git@v0.1.0
```

必要なもの: [Python](https://www.python.org) ≥ 3.9 と [Rust ツールチェーン](https://rustup.rs)
(rustc ≥ 1.88)。pip がリポジトリを clone し、同梱の `optopus` サブモジュールを自動で取得して、
maturin で拡張をビルドします。追加の手順はありません。

このページの残りは、clone したリポジトリで開発環境を構築する手順です。

(prerequisites)=
## 前提条件

- [Python](https://www.python.org) ≥ 3.9
- [uv](https://github.com/astral-sh/uv) — Python の環境と依存関係の管理ツール
- [Rust ツールチェーン](https://rustup.rs) (`cargo`、`rustc`) — 拡張のコンパイルに必要

Rust ライブラリ [`optopus`](https://github.com/trash-iine/optopus) は git サブモジュールとして
`vendor/optopus` に同梱しています (バインディングのクレートはパス依存
`optopus = { path = "vendor/optopus" }` で参照します)。そのため、サブモジュールごと clone します。

```bash
$ git clone --recurse-submodules https://github.com/trash-iine/optopus-py.git
```

サブモジュールなしで clone 済みの場合は、次のコマンドで初期化します。

```bash
$ git submodule update --init
```

(set-up-the-development-environment)=
## 開発環境の構築

`optopus-py` ディレクトリで次を実行します。

```bash
$ uv sync --dev
```

`.venv/` が作られ、maturin で拡張がコンパイルされて環境にインストールされます。ドキュメント用の
ツール (Sphinx、furo、myst-parser、invoke) も入ります。

(verify-the-installation)=
## インストールの確認

```bash
$ uv run python -c "import optopus; print(optopus.MaxCut.from_edges([(0, 1, 1.0)]))"
MaxCut(num_vertices=2, num_edges=1)
```

(standalone-build-without-uv)=
## uv を使わないビルド

素の仮想環境を使いたい場合は、maturin で直接ビルドしてインストールできます。

```bash
$ python -m venv .venv
$ source .venv/bin/activate
$ pip install maturin
$ maturin develop --release
```

(build-the-documentation)=
## ドキュメントのビルド

```bash
$ uv run invoke docs
```

HTML は `docs/build/html/` に出力され、日本語版はその下の `docs/build/html/ja/` に出力されます。
ブラウザで `docs/build/html/ja/index.html` を開くと読めます。
