# QuickAccent on Arch / Omarchy (AUR)

`quickaccent-bin` packages the prebuilt tarball of a tagged release. The
tarball is pinned by sha256 in the PKGBUILD, and every release asset carries
a Sigstore build-provenance attestation you can check independently:

```bash
gh attestation verify quickaccent-linux-x86_64.tar.gz --repo victormasson/QuickAccent
```

## Install

```bash
yay -S quickaccent-bin           # or: omarchy pkg add quickaccent-bin
sudo usermod -aG input "$USER"   # then reboot once
systemctl --user enable --now quickaccent
```

## Remove

```bash
systemctl --user disable --now quickaccent
sudo pacman -Rns quickaccent-bin
sudo gpasswd -d "$USER" input    # optional
```

## Publishing a new version (maintainer)

After `git tag vX.Y.Z && git push origin vX.Y.Z` has produced the release:

```bash
cd dist/arch
sed -i "s/^pkgver=.*/pkgver=X.Y.Z/; s/^pkgrel=.*/pkgrel=1/" PKGBUILD
updpkgsums                          # pacman-contrib: pins the tarball sha256
makepkg --printsrcinfo > .SRCINFO
namcap PKGBUILD && makepkg -sfi     # build + install locally to test
```

Then push `PKGBUILD`, `quickaccent.install` and `.SRCINFO` to the AUR
repository (`ssh://aur@aur.archlinux.org/quickaccent-bin.git`).
