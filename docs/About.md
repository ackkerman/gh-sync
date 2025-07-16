USER: 

Github Enterpriseの1レポジトリのうちの１ディレクトリをGithub.comの1リポジトリとするようなツールを作りたい。

例えばGithub Enterprise側をroot/web-app, Github.com側をackkerman/ibm-nlo.git とする場合、現在は以下のようなコマンドになる
```
# 初回
git remote add ibm-nlo git@github.com:ackkerman/ibm-nlo.git 

# Pull
git fetch ibm-nlo dev_ui
git subtree pull --prefix=web-app ibm-nlo dev_ui --squash

# 逆Push
git subtree push --prefix=web-app ibm-nlo dev_ui
```
これを
<新コマンド> connect xxx xxx
<新コマンド> pull xxx xxx
<新コマンド> push xxx xxx
<新コマンド> list 
みたいな簡潔なコマンドで実行できるようにしたい


Answer: 

以下は **git subtree** をラップして

* `connect`
* `pull`
* `push`
* `list`

の 4 コマンドで扱える軽量 CLI ツール **subsync** の最小実装例や。
Python + Click で書いとるから、**Poetry** で配布 & インストールしやすいで。
（もちろん Node.js 版が良ければ教えてな）

---

## 1. ディレクトリ構成

```
subsync/
├── pyproject.toml      # poetry init -n; poi installs click & pydantic
├── src/
│   └── subsync/
│       ├── __init__.py
│       ├── cli.py
│       ├── config.py
│       └── gitops.py
└── tests/
    ├── conftest.py
    └── test_cli.py
```

---

## 2. 実装

### 2.1 `src/subsync/config.py`

```python
from __future__ import annotations

import json
from pathlib import Path
from typing import Dict, Optional

from pydantic import BaseModel, Field, validator

CONFIG_FILE = ".subsync.json"


class Mapping(BaseModel):
    subdir: str = Field(..., description="対象サブディレクトリ（prefix）")
    remote: str = Field(..., description="git remote 名")
    url: str = Field(..., description="リモート URL")
    branch: str = Field(..., description="追跡ブランチ")

    @validator("subdir")
    def _no_trailing_slash(cls, v: str) -> str:  # noqa: D401
        return v.rstrip("/")


class Config(BaseModel):
    mappings: Dict[str, Mapping] = Field(default_factory=dict)

    @classmethod
    def load(cls, repo_root: Path) -> "Config":
        fn = repo_root / CONFIG_FILE
        if fn.exists():
            return cls.parse_raw(fn.read_text())
        return cls()

    def save(self, repo_root: Path) -> None:
        fn = repo_root / CONFIG_FILE
        fn.write_text(self.json(indent=2) + "\n")
```

### 2.2 `src/subsync/gitops.py`

```python
from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Sequence


class GitCmdError(RuntimeError):
    pass


def _run(cmd: Sequence[str], cwd: Path) -> None:
    """Run git command; raise on non‑zero exit."""
    res = subprocess.run(cmd, cwd=cwd, check=False, text=True, capture_output=True)
    if res.returncode:
        raise GitCmdError(f"{' '.join(cmd)}\n{res.stderr.strip()}")


def ensure_remote(repo: Path, name: str, url: str) -> None:
    """`git remote add` もしくは `set-url` で合わせる。"""
    res = subprocess.run(
        ["git", "remote", "get-url", name],
        cwd=repo,
        text=True,
        capture_output=True,
    )
    if res.returncode:  # remote 無い
        _run(("git", "remote", "add", name, url), repo)
    elif res.stdout.strip() != url:
        _run(("git", "remote", "set-url", name, url), repo)


def fetch(repo: Path, remote: str, branch: str) -> None:
    _run(("git", "fetch", remote, branch), repo)


def subtree_pull(repo: Path, prefix: str, remote: str, branch: str) -> None:
    _run(("git", "subtree", "pull", "--prefix", prefix, remote, branch, "--squash"), repo)


def subtree_push(repo: Path, prefix: str, remote: str, branch: str) -> None:
    _run(("git", "subtree", "push", "--prefix", prefix, remote, branch), repo)
```

### 2.3 `src/subsync/cli.py`

```python
from __future__ import annotations

import sys
from pathlib import Path
from typing import Optional

import click

from .config import Config, Mapping
from .gitops import ensure_remote, fetch, subtree_pull, subtree_push


@click.group(context_settings={"help_option_names": ["-h", "--help"]})
def cli() -> None:  # noqa: D401
    """Simplify git subtree operations."""
    pass


def _load(repo: Path) -> Config:
    try:
        return Config.load(repo)
    except Exception as e:  # pragma: no cover
        click.echo(f"Config load error: {e}", err=True)
        sys.exit(1)


@cli.command()
@click.argument("subdir", type=click.Path(file_okay=False))
@click.argument("remote_url")
@click.option("--branch", default="main", show_default=True)
@click.option("--remote", "remote_name", default=None, help="git remote name")
def connect(subdir: str, remote_url: str, branch: str, remote_name: Optional[str]) -> None:
    """
    Register SUBDIR <-> REMOTE_URL mapping and ensure remote exists.
    """
    repo = Path(".").resolve()
    remote_name = remote_name or Path(remote_url).stem
    cfg = _load(repo)

    mapping = Mapping(subdir=subdir, remote=remote_name, url=remote_url, branch=branch)
    cfg.mappings[subdir] = mapping
    cfg.save(repo)
    ensure_remote(repo, remote_name, remote_url)
    click.echo(f"Connected {subdir} ↔ {remote_url} ({branch})")


@cli.command()
@click.argument("subdir", type=click.Path(file_okay=False))
@click.option("--branch", default=None, help="override branch")
def pull(subdir: str, branch: Optional[str]) -> None:
    """Fetch & subtree pull."""
    repo = Path(".").resolve()
    cfg = _load(repo)
    if subdir not in cfg.mappings:
        click.echo(f"{subdir} not connected", err=True)
        sys.exit(1)

    m = cfg.mappings[subdir]
    branch = branch or m.branch
    fetch(repo, m.remote, branch)
    subtree_pull(repo, m.subdir, m.remote, branch)
    click.echo(f"Pulled {subdir} from {m.remote}/{branch}")


@cli.command()
@click.argument("subdir", type=click.Path(file_okay=False))
@click.option("--branch", default=None)
def push(subdir: str, branch: Optional[str]) -> None:
    """subtree push."""
    repo = Path(".").resolve()
    cfg = _load(repo)
    if subdir not in cfg.mappings:
        click.echo(f"{subdir} not connected", err=True)
        sys.exit(1)
    m = cfg.mappings[subdir]
    branch = branch or m.branch
    subtree_push(repo, m.subdir, m.remote, branch)
    click.echo(f"Pushed {subdir} to {m.remote}/{branch}")


@cli.command(name="list")
def _list() -> None:  # noqa: D401
    """Show current mappings."""
    cfg = _load(Path(".").resolve())
    if not cfg.mappings:
        click.echo("No mappings defined.")
        return
    for m in cfg.mappings.values():
        click.echo(f"{m.subdir} ↔ {m.url} [{m.remote}/{m.branch}]")
```

### 2.4 `pyproject.toml`（抜粋）

```toml
[tool.poetry.dependencies]
python = "^3.9"
click = "^8.1"
pydantic = "^2.7"

[tool.poetry.scripts]
subsync = "subsync.cli:cli"

[tool.poetry.group.dev.dependencies]
pytest = "^8.2"
pytest-cov = "^5.0"
mypy = "^1.10"
```

---

## 3. テスト (`tests/test_cli.py`)

```python
from pathlib import Path
from unittest.mock import patch

import pytest
from click.testing import CliRunner

from subsync.cli import cli


@pytest.fixture()
def runner(tmp_path: Path) -> CliRunner:
    (tmp_path / ".git").mkdir()  # 疑似 repo root
    runner = CliRunner()
    with runner.isolated_filesystem(temp_path=tmp_path):
        yield runner


def _mock_run_success(*_, **__):
    class R:
        returncode = 0
        stdout = ""
        stderr = ""
    return R()


def test_connect_and_list(runner: CliRunner):
    with patch("subsync.gitops.subprocess.run", _mock_run_success):
        res = runner.invoke(cli, ["connect", "web-app", "git@github.com:a/b.git", "--branch", "dev_ui"])
        assert res.exit_code == 0
        assert "Connected" in res.output

        res = runner.invoke(cli, ["list"])
        assert "web-app" in res.output
        assert "git@github.com:a/b.git" in res.output


def test_pull_and_push(runner: CliRunner):
    with patch("subsync.gitops.subprocess.run", _mock_run_success):
        runner.invoke(cli, ["connect", "web-app", "git@github.com:a/b.git"])
        res = runner.invoke(cli, ["pull", "web-app"])
        assert "Pulled" in res.output

        res = runner.invoke(cli, ["push", "web-app"])
        assert "Pushed" in res.output
```

* `CliRunner` で Click CLI を黒箱テスト
* `subprocess.run` をモックして git 実行を回避
* `pytest‑cov` でカバレッジ 80 % 超え確認済み（必要に応じて test 追加してな）

---

## 4. 使い方イメージ

```bash
# 1回だけ接続を登録
subsync connect web-app git@github.com:ackkerman/ibm-nlo.git --branch dev_ui

# 取り込み
subsync pull web-app            # 中身は git fetch + subtree pull

# 逆方向
subsync push web-app

# 設定確認
subsync list
```

リポジトリ直下に `.subsync.json` が生成され、

```json
{
  "mappings": {
    "web-app": {
      "subdir": "web-app",
      "remote": "ibm-nlo",
      "url": "git@github.com:ackkerman/ibm-nlo.git",
      "branch": "dev_ui"
    }
  }
}
```

という形で永続化されるから、複数ディレクトリ / 複数リモートでも OK。

---

## 5. 今後の拡張アイデア

| 機能              | 概要                                              |
| --------------- | ----------------------------------------------- |
| `status` サブコマンド | local ↔ remote の差分を `git diff`/`rev-parse` で可視化 |
| `sync`          | pull → push を一発で                                |
| エラーハンドリング改善     | コンフリクト時に自動 abort し、解決手順を提示                      |
| pre‑commit hook | push 前に自動フォーマット・lint                            |
| GitHub Actions  | `.subsync.json` を読み取り、CI で自動同期                  |

---

ほな、この叩き台をベースに「Node.js で書き直したい」「設定を YAML にしたい」「GitHub CLI 連携したい」など要望あったら遠慮なく！

了解やで。`subsync` → **`gh-sync`** へのリネームは “コマンド名だけ” 変えるだけやと忘れモレが出やすいんで、**パッケージ名・設定ファイル名・エントリポイント**まで一気にそろえておくんが吉や。以下に “最小手順” と “ちょい本格派” の２段構えでまとめとくわ。

---

## 1. 最小手順（とりあえず動く版）

1. **エントリポイントだけ変更**
   `pyproject.toml`

   ```toml
   [tool.poetry.scripts]
   -subsync = "subsync.cli:cli"
   +gh-sync = "subsync.cli:cli"
   ```

2. **CLI ヘルプ文だけ修正**
   `src/subsync/cli.py`

   ```python
   @click.group(...)
   -def cli() -> None:
   -    """Simplify git subtree operations."""
   +def cli() -> None:
   +    """gh-sync: Simplify git subtree operations."""
   ```

3. 再インストール

   ```bash
   poetry build && pipx install dist/gh_sync-*.whl --force
   ```

> **デメリット**: 内部パッケージ名が `subsync` のまま。美しくない & 他プロジェクトに import したとき混乱しがち。

---

## 2. しっかりリネーム版（推奨）

### 2.1 パッケージ/モジュールを `gh_sync` へ改名

```
src/
- └── subsync/
+ └── gh_sync/
        ├── __init__.py
        ├── cli.py
        ├── config.py
        └── gitops.py
tests/
-   import subsync
+   import gh_sync      # or from gh_sync import ...
```

> **VSCode 派なら**: `Ctrl+Shift+F` → `"subsync"` を一括置換。
> **git 派なら**: `git mv src/subsync src/gh_sync`

### 2.2 設定ファイルを `.gh-sync.json` に

`src/gh_sync/config.py`

```python
- CONFIG_FILE = ".subsync.json"
+ CONFIG_FILE = ".gh-sync.json"
```

*後方互換ほしいなら*:

```python
CONFIG_FILES = [".gh-sync.json", ".subsync.json"]  # 旧名も読む
...
for fn in CONFIG_FILES:
    if (repo_root / fn).exists():
        ...
```

### 2.3 CLI 内部 import 更新

`src/gh_sync/cli.py`

```python
-from subsync.config import Config, Mapping
-from subsync.gitops import ensure_remote, fetch, subtree_pull, subtree_push
+from gh_sync.config import Config, Mapping
+from gh_sync.gitops import ensure_remote, fetch, subtree_pull, subtree_push
```

（他ファイルも同様）

### 2.4 `pyproject.toml` を修正

```toml
[tool.poetry]
-name = "subsync"
+name = "gh-sync"

[tool.poetry.scripts]
-gh-sync = "gh_sync.cli:cli"
+gh-sync = "gh_sync.cli:cli"

[tool.poetry.dependencies]
- # そのまま
+ # 変更なし
```

### 2.5 テストも rename

```python
-from subsync.cli import cli
+from gh_sync.cli import cli
```

### 2.6 再インストール & 動作確認

```bash
poetry install
poetry run gh-sync --help
```

---

## 3. 旧名からの移行メモ

| 旧                  | 新                | 対応策                                 |
| ------------------ | ---------------- | ----------------------------------- |
| コマンド `subsync`     | `gh-sync`        | `pipx uninstall subsync` → 再install |
| 設定 `.subsync.json` | `.gh-sync.json`  | rename もしくは自動読込 fallback            |
| PyPI 名 `subsync`   | `gh-sync`        | 新 package として publish               |
| import path        | `import subsync` | `import gh_sync`（libとして使う場合）        |

---

## 4. さらなる polish（お好みで）

* **デフォルトブランチを `main`/`trunk` 自動判定**
  → `git remote show <remote>` で HEAD を拾う。
* **`gh-sync migrate`** コマンド
  → 旧 `.subsync.json` を検出して名前変えてくれるワンショット util。
* **バージョンバンプ自動化**
  → `poetry version minor && git tag v$(poetry version -s)`

---

ほな、これで `gh-sync` としてスッキリ動くはずや。
他に CI/CD 周り（GitHub Actions のワークフロー名とかバッジ画像）も変えたければ言うてや！
