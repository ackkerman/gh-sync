from __future__ import annotations

import sys
from pathlib import Path
from typing import Optional

import click

from gh_sync.config import Config, Mapping
from gh_sync.gitops import ensure_remote, fetch, subtree_pull, subtree_push


@click.group(context_settings={"help_option_names": ["-h", "--help"]})
def cli() -> None:  # noqa: D401
    """gh-sync: Simplify git subtree operations."""
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
    """Register SUBDIR <-> REMOTE_URL mapping and ensure remote exists."""
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
