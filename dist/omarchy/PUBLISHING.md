# Publishing this plugin to omarchyplugins.com

The marketplace requires `manifest.json` in the **repository root**, so this
directory has to be pushed as its own repository.

```bash
# from a clone of QuickAccent
cp -r dist/omarchy /tmp/omarchy-plugin-quickaccent
cd /tmp/omarchy-plugin-quickaccent
rm PUBLISHING.md
git init -b main && git add -A
git commit -m "QuickAccent plugin for Omarchy"
gh repo create victormasson/omarchy-plugin-quickaccent --public --source=. --push
```

Verify on an Omarchy machine before submitting:

```bash
cp -r . ~/.config/omarchy/plugins/io.github.victormasson.quickaccent
omarchy plugin validate ~/.config/omarchy/plugins/io.github.victormasson.quickaccent
qmllint -I "$OMARCHY_PATH/shell" BarWidget.qml
omarchy plugin list --json | jq '.[] | select(.id == "io.github.victormasson.quickaccent")'
```

Publish a new version: bump `manifest.json`, copy this directory over the
plugin repo, commit and push. For an open submission, edit the issue body
afterwards so the marketplace rescans the new HEAD.

Then open the submission issue:
<https://github.com/omacom/omarchy-plugin-marketplace/issues/new?template=submit-plugin.yml>

Form values:

| Field | Value |
|-------|-------|
| Repository URL | `https://github.com/victormasson/omarchy-plugin-quickaccent` |
| Category | System |
| Tags | Hyprland, System, Quickshell |
| Maintainer notes | Front-end for the QuickAccent daemon, installed separately from the AUR (`quickaccent-bin`, tarball pinned by sha256, release assets carry a Sigstore build-provenance attestation; no script is downloaded and executed). Needs membership of group `input` and access to `/dev/uinput` to grab keys and inject accents; the widget itself only calls `systemctl --user`. Enables the xkb option `quickaccent:accents` via `hyprctl keyword input:kb_options` at runtime and does not modify `hyprland.conf`. MIT. |

Checklist: repository public with instructions, license and dependencies
documented, ownership confirmed, plugin respects user configuration.
