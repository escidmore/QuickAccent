# QuickAccent on macOS

## Install (prebuilt, no Rust)

```bash
curl -fsSL https://raw.githubusercontent.com/victormasson/QuickAccent/master/dist/macos/install.sh | bash
open ~/Applications/QuickAccent.app
```

Grant **Accessibility** (System Settings → Privacy & Security → Accessibility).
If a stale QuickAccent entry is already listed (an older install), remove it
and add the new one — the grant is bound to the bundle's code signature.

Always launch through Launch Services (`open`, Finder, or a LaunchAgent that
runs `open`). If launchd execs `Contents/MacOS/quickaccent` directly, macOS
ignores the Accessibility grant and the app logs `Failed to create CGEventTap`.

Downloads `QuickAccent-macos-universal.tar.gz` from the rolling [`continuous`](https://github.com/victormasson/QuickAccent/releases/tag/continuous) release (arm64 + x86_64).

| Env | Default |
|-----|---------|
| `GITHUB_REPO` | `victormasson/QuickAccent` |
| `QUICKACCENT_VERSION` | `continuous` |
| `PREFIX` | `~/Applications` |
| `INSTALL_FROM_SOURCE` | `0` |

## Start at login

```bash
cp dist/macos/com.quickaccent.app.plist ~/Library/LaunchAgents/
# edit the app path inside if you installed to ~/Applications
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.quickaccent.app.plist
```

Logs go to `/tmp/quickaccent.log` / `/tmp/quickaccent.err`. Stop with
`launchctl bootout gui/$(id -u)/com.quickaccent.app`.

## Homebrew (from source)

```bash
brew install --HEAD ./dist/brew/Formula/quickaccent.rb
```

## From source

```bash
INSTALL_FROM_SOURCE=1 ./dist/macos/install.sh
# or
cargo build --release
```
