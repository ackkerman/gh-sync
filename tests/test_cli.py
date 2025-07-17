from pathlib import Path
from unittest.mock import patch

from click.testing import CliRunner

from gh_sync.cli import cli


def _mock_run_success(*_, **__):
    class R:
        returncode = 0
        stdout = ""
        stderr = ""
    return R()


def _mock_run_fail(*args, **kwargs):
    class R:
        returncode = 1
        stdout = ""
        stderr = "boom"

    return R()


def test_connect_and_list(runner: CliRunner):
    with patch("gh_sync.gitops.subprocess.run", _mock_run_success):
        res = runner.invoke(cli, ["connect", "web-app", "git@github.com:a/b.git", "--branch", "dev_ui"])
        assert res.exit_code == 0
        assert "Connected" in res.output

        res = runner.invoke(cli, ["list"])
        assert "web-app" in res.output
        assert "git@github.com:a/b.git" in res.output


def test_pull_and_push(runner: CliRunner):
    with patch("gh_sync.gitops.subprocess.run", _mock_run_success):
        runner.invoke(cli, ["connect", "web-app", "git@github.com:a/b.git"])
        res = runner.invoke(cli, ["pull", "web-app"])
        assert "Pulled" in res.output

        res = runner.invoke(cli, ["push", "web-app"])
        assert "Pushed" in res.output


def test_pull_first_time_adds_subtree(runner: CliRunner):
    calls = []

    def _mock_run(cmd, cwd=None, check=False, text=True, capture_output=True):
        calls.append(cmd)

        class R:
            stdout = ""
            stderr = ""

        r = R()
        if tuple(cmd[:3]) == ("git", "subtree", "pull"):
            r.returncode = 1
            r.stderr = "fatal: can't squash-merge: 'web-app' was never added."
        else:
            r.returncode = 0
        return r

    with patch("gh_sync.gitops.subprocess.run", _mock_run):
        runner.invoke(cli, ["connect", "web-app", "git@github.com:a/b.git"])
        res = runner.invoke(cli, ["pull", "web-app"])
        assert res.exit_code == 0
        assert "Pulled" in res.output
        assert any(tuple(cmd[:3]) == ("git", "subtree", "add") for cmd in calls)


def test_pull_failure_shows_message(runner: CliRunner):
    with patch("gh_sync.gitops.subprocess.run", _mock_run_success):
        runner.invoke(cli, ["connect", "web-app", "git@github.com:a/b.git"])

    with patch("gh_sync.gitops.subprocess.run", _mock_run_fail):
        res = runner.invoke(cli, ["pull", "web-app"])
        assert res.exit_code == 1
        assert "git fetch" in res.output
        assert "boom" in res.output
        assert "Traceback" not in res.output


def test_push_failure_shows_message(runner: CliRunner):
    with patch("gh_sync.gitops.subprocess.run", _mock_run_success):
        runner.invoke(cli, ["connect", "web-app", "git@github.com:a/b.git"])

    with patch("gh_sync.gitops.subprocess.run", _mock_run_fail):
        res = runner.invoke(cli, ["push", "web-app"])
        assert res.exit_code == 1
        assert "git subtree push" in res.output
        assert "boom" in res.output
        assert "Traceback" not in res.output
