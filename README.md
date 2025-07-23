# gh-sync

Synchronize a subdirectory of your repository with an external repository using a simple GitHub CLI extension. gh-sync wraps `git subtree` so that you can easily pull and push changes while keeping track of the mapping in your Git configuration.

## Features

- **connect** – register a subdirectory ↔ remote URL mapping
- **pull** – fetch from the remote and update the subtree
- **push** – push local changes in the subtree back to the remote
- **list** – show current mappings

These commands make it straightforward to synchronize only a portion of a large repository with another repository.

## Installation

Install locally using Cargo:

```bash
cargo install --path .
```

Or install as a GitHub CLI extension:

```bash
gh extension install ackkerman/gh-sync
```

## Usage

```bash
# Register a mapping (once)
gh sync connect web-app git@github.com:ackkerman/nlo.git --branch dev_ui

# Pull updates from the remote
gh sync pull web-app

# Push local changes back
gh sync push web-app

# View mappings
gh sync list
```

Mappings are stored in your repository's Git configuration under the `gh-sync` prefix. Multiple subdirectories can be managed.

## License

MIT
