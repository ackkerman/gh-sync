from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Sequence


class GitCmdError(RuntimeError):
    pass


def _run(cmd: Sequence[str], cwd: Path) -> None:
    """Run git command; raise on non-zero exit."""
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
    """Run ``git subtree pull``; auto-add prefix if missing."""
    try:
        _run(
            (
                "git",
                "subtree",
                "pull",
                "--prefix",
                prefix,
                remote,
                branch,
                "--squash",
            ),
            repo,
        )
    except GitCmdError as e:
        if "use 'git subtree add'" in str(e):
            _run(
                (
                    "git",
                    "subtree",
                    "add",
                    "--prefix",
                    prefix,
                    remote,
                    branch,
                    "--squash",
                ),
                repo,
            )
        else:
            raise



def subtree_push(repo: Path, prefix: str, remote: str, branch: str) -> None:
    _run(("git", "subtree", "push", "--prefix", prefix, remote, branch), repo)
