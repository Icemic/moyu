# 末語（Moyu） - クロスプラットフォーム・ビジュアルノベルエンジン

[简体中文](README.md) | [English](README_EN.md) | **日本語**

Rust をコアとし、React でインターフェースと演出を構築するクロスプラットフォーム・ビジュアルノベルエンジンです。

<p>
  <a href="https://opensource.org/licenses/MPL-2.0"><img alt="MPL-2.0 License" src="https://img.shields.io/badge/license-MPL%202.0-blue.svg"></a>
  <a href="https://github.com/Icemic/moyu/actions"><img alt="Rust CI" src="https://github.com/Icemic/moyu/actions/workflows/build.yml/badge.svg"></a>
  <a href="https://github.com/Icemic/moyu/pulls"><img alt="PRs Welcome" src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat"></a>
  <a href="https://discord.gg/wmTekCNarG"><img alt="Discord" src="https://img.shields.io/discord/1260706796646170765?label=Discord&logo=discord&logoColor=white"></a>
  <a href="http://qm.qq.com/cgi-bin/qm/qr?_wv=1027&k=dcB58s03NbyIENYYtp0IHa8aTcUzlBF4&authKey=cgKWlgzqOhczlLbJbGo%2F1wLiUzH%2FMXNSTxz%2BNhDjMufuw0egSin7eqZKoRD7vF4l&noverify=0&group_code=293602841"><img alt="QQ" src="https://img.shields.io/badge/QQ-293602841-blue?logo=qq&logoColor=white"></a>
</p>

**React でビジュアルノベルを書く | プログレッシブなクロスプラットフォームエンジン | MPL-2.0**

末語はモダンなビジュアルノベル開発のために、Rust コアと JS/React の開発スタイルを組み合わせ、迅速なプロトタイピングから本格的なカスタマイズまで、段階的な開発体験をクリエイターに提供します。

詳しい紹介・チュートリアル・ドキュメントは公式サイトをご覧ください：<https://momoyu.ink>。

## 特長

- **一貫したクロスプラットフォーム対応**：Windows、macOS、Linux、Android、iOS、Web に対応し、一度書けばどこでも動作します。
- **複数のグラフィックスバックエンド**：Vulkan / Metal / DX12 / OpenGL を切り替え可能。
- **高度にカスタマイズ可能なインターフェース**：React であらゆる UI とシステムを構築し、成熟したコミュニティのリソースとツールを活用できます。
- **プログレッシブで柔軟**：標準フレームワークから Rust 層への深い拡張まで、複雑さを段階的に解放します。
- **オープンソースかつ商用フレンドリー**：MPL-2.0 ライセンスに基づき、商用プロジェクトを含め無料で利用できます。

### レイヤードアーキテクチャ

- **Rust 層**：リソース管理 / グラフィックスレンダリング / オーディオシステム / ネイティブプラグイン。
- **JavaScript 層**：React コンポーネント / ストーリーロジック / アニメーション制御。

## リポジトリ構成

- `crates/` — Rust で実装されたエンジンコア、ランタイム、ノード、プラットフォーム抽象など。
- `packages/` — 上位の JavaScript / TypeScript：`@momoyu-ink/kit`（React SDK）、`@momoyu-ink/cli`（CLI）など。

## クイックスタート

エンジン本体は、付属の標準フレームワークを通じて利用します。公式フレームワークのリポジトリにアクセスし、その手順に従ってクローン・インストール・実行してください：

<https://github.com/DeepSpaceMill/framework>

インストール、アセットの配置、シナリオの記述に関する詳しいガイドは公式サイトをご覧ください：<https://momoyu.ink>。

## コントリビュート

ドキュメントの改善や国際化、テンプレート開発、エンジン機能の拡張、新しいプラットフォーム対応、パフォーマンス最適化など、あらゆる貢献を歓迎します。始める前に[コントリビューションガイド](CONTRIBUTING.md)をお読みください。

## コミュニティ

質問やアイデアがあれば、Discord または QQ グループでお気軽に交流してください。

## ライセンス

特に明記がない限り、本プロジェクトは Mozilla Public License v2.0（MPL-2.0）に基づきます。詳細は各 `Cargo.toml` または `package.json` ファイルおよび [LICENSE](LICENSE.txt) ファイルをご覧ください。

一部のコンテンツは MIT ライセンスに基づきます。該当ディレクトリ内の `package.json` および LICENSE ファイルをご確認ください。
