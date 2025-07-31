# About gh-sync

gh-sync is a lightweight CLI tool for synchronizing a subdirectory of one Git repository with another using `git subtree` under the hood. Configuration is stored in the repository's Git config so that mappings can be reused easily.

## Current Commands

- `connect <subdir> <remote_url> [--branch <name>] [--remote <remote>]`  
  Register a mapping between a local subdirectory and a remote repository/branch.
- `pull <subdir> [--branch <name>] [-m <message>]`
  Fetch from the configured remote and update the subtree using the given merge commit message.
- `push <subdir> [--branch <name>] [-m <message>]`
  Push local changes in the subtree back to the remote using the provided commit message.
- `remove <subdir>`  
  Delete the mapping from Git config.
- `list`  
  Show all registered mappings.

Mappings are saved under the `gh-sync.*` keys inside `.git/config`. Each section stores the remote name, URL and branch to use for synchronization.

## Example

```bash
# Register once
gh sync connect web-app git@github.com:example/remote.git --branch main

# Fetch and merge remote changes
gh sync pull web-app -m "Update from remote"

# Push local updates back
gh sync push web-app -m "Sync subtree"

# Remove mapping
gh sync remove web-app

# Show mappings
gh sync list
```

This repository contains the Rust implementation of the tool. Unit tests use temporary repositories with shimmed `git` binaries to verify behaviour without touching real repositories.
