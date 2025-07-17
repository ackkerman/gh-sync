# gh-sync

GitHub Enterprise ↔ GitHub.com subtree sync tool (Rust)

```bash
cargo install --path . # or `cargo build --release`
```

Then use `gh-sync` to register and sync subtrees:

```bash
# register once
gh-sync connect web-app git@github.com:ackkerman/ibm-nlo.git --branch dev_ui

# fetch updates
gh-sync pull web-app

# push back
gh-sync push web-app

# show mappings
gh-sync list
```
