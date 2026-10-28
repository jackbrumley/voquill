# Release Process

This is the single, ordered procedure for releasing Voquill. Follow it exactly.
GitHub Actions builds, uploads, verifies and install-tests every asset; a
release takes two commands to start and one to publish.

Release identity (tag, title, asset names, download links) is consumed by
`install.sh`, `install-macos.sh`, `install.ps1` and the in-app updater, so a
format mistake is not cosmetic: it breaks updates and one-line installs for
every user.

## Format Rules

Every place the version appears, using `1.8.0` as the example:

| Where | Format | Example |
|---|---|---|
| `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` | `X.Y.Z` | `1.8.0` |
| Version bump commit message | `chore: bump version to X.Y.Z` | `chore: bump version to 1.8.0` |
| Git tag | `vX.Y.Z` | `v1.8.0` |
| GitHub release title | `vX.Y.Z` (no "Voquill") | `v1.8.0` |
| First line of the release notes | `# Voquill vX.Y.Z` | `# Voquill v1.8.0` |
| Asset file names | `voquill-X.Y.Z-<os>-<arch>[-setup].<ext>` | `voquill-1.8.0-linux-x64.deb` |
| Download URLs | `.../releases/download/vX.Y.Z/voquill-X.Y.Z-...` | tag keeps the `v`, file name does not |

Rules:
- The version is plain `MAJOR.MINOR.PATCH`. No `-beta.1` or other suffix: the
  extra hyphens collide with the asset name separators. The release scripts
  enforce this.
- Asset names, the expected asset list and download links all come from
  `scripts/release-assets.mjs`. Never type or edit them by hand.

A complete release carries six assets:
- `voquill-X.Y.Z-windows-x64-setup.exe`
- `voquill-X.Y.Z-windows-x64.msi`
- `voquill-X.Y.Z-linux-x64.deb`
- `voquill-X.Y.Z-linux-x64.rpm`
- `voquill-X.Y.Z-linux-x64.AppImage`
- `voquill-X.Y.Z-macos-arm64.zip`

## Branches

Development happens on `dev`. A release is tagged on `dev`. Promotion
fast-forwards `main` to the release tag, so `main` always equals the latest
published release. GitHub Pages serves the website and install scripts from
`main:/docs`, so promotion is also what deploys installer changes.

Never commit to `main` directly; it only moves by promotion.

## 1. Prepare and Tag

On a `dev` checkout:
```bash
npm run release:prepare -- X.Y.Z
git push origin dev vX.Y.Z
```

`release:prepare` sets the version in the three version files, refreshes both
lockfiles, commits exactly those five files as `chore: bump version to X.Y.Z`
and creates the `vX.Y.Z` tag. Other uncommitted work is left untouched.

Pushing the tag starts the **Release** workflow (`.github/workflows/release.yml`):
1. **Prepare** checks the tag matches `tauri.conf.json` and creates the
   **pre-release** with the release notes skeleton.
2. **Build** runs Linux, Windows and macOS in parallel. Linux builds inside the
   pinned Ubuntu 22.04 container (`npm run release:linux`, glibc 2.35 floor),
   Windows and macOS on GitHub's runners. Each job names its files with
   `package-release.mjs` and uploads them to the pre-release.
3. **Verify** fails unless the release carries exactly the six assets, each
   with the SHA-256 digest the installers check.
4. **Install test** installs the pre-release on Linux (.deb and AppImage),
   Windows (setup.exe and MSI) and macOS with the installer scripts from the
   tag, and checks the installed app.

A full run takes roughly 60–90 minutes. Follow it with:
```bash
gh run watch "$(gh run list --workflow release.yml --limit 1 --json databaseId --jq '.[0].databaseId')" --exit-status
```

A pre-release is invisible to `/releases/latest`, so the in-app updater and the
one-line installers keep offering the previous release until promotion.

## 2. Write the Release Notes

The pre-release starts with the generated skeleton. Regenerate it locally to see
the source commits:
```bash
npm run --silent release:notes > release-notes.md
```

stdout is the notes skeleton; the commits since the previous tag are printed to
the terminal (stderr) as source material. Fill in every `TODO` line, keeping
this structure (see v1.7.3 and v1.7.5 for complete examples), then publish the
notes to the pre-release:
```bash
gh release edit vX.Y.Z --notes-file release-notes.md
```

```markdown
# Voquill vX.Y.Z

Voquill vX.Y.Z is a ... release, <one paragraph summarising the headline changes>.

---

## What's New

### Theme Name
- Short Label: One user-facing sentence about the change.

---

### Another Theme
- Short Label: One user-facing sentence about the change.

---

## Downloads

<generated block: all six links, never edited>
```

Content rules:
- Cover everything since the last release that was promoted to latest, not just
  since the previous tag. A pre-release that was never promoted (e.g. v1.7.4)
  was never offered to users, so its changes belong in these notes too: run
  `git log --no-merges --format='- %s' vLAST_LATEST..vX.Y.Z` for the full list.
- Write for users, not developers: describe what changed for them. No commit
  hashes, file paths or internal type names.
- Group changes into themed `###` sections, separated by `---`. Bullets are
  `- Label: sentence` with no bold.
- If users must do anything after updating (re-select a setting, re-grant a
  permission), say so.
- Leave out commits that don't affect users (docs, refactors, CI) unless they
  change something users see.
- The Downloads block always lists all six assets. Never remove or reorder links.
- Wrap anything containing angle brackets in backticks (e.g.
  `` `config.json.invalid-<date>` ``); GitHub otherwise hides `<...>` as HTML.

## 3. Promote

Once the workflow is green and the notes are written:
```bash
npm run release:promote
```

`release:promote` refuses unless the pre-release carries all six assets with
digests and its notes contain no `TODO`. It then marks the release as latest
and fast-forwards `main` to the tag, which deploys the website and installers.
From that moment the in-app updater and the one-line installers serve the new
version.

## 4. Verify

- An installed older version offers the update (Settings → General → Check for
  Updates), and the update installs and relaunches.
- The one-line installers on voquill.org install the new version.

## When Something Fails

- **A build or install test failed for a transient reason** (network, runner):
  re-run only the failed jobs: `gh run rerun <run-id> --failed`. Uploads use
  `--clobber`, so re-runs replace partial assets.
- **A build failed because of the code:** the release was never promoted, so
  nobody received it. Fix the problem on `dev`, then discard the tag and
  pre-release and tag again:
  ```bash
  gh release delete vX.Y.Z --cleanup-tag --yes
  git tag -d vX.Y.Z
  git tag vX.Y.Z
  git push origin dev vX.Y.Z
  ```

## Dry Run

To check that every platform still builds and packages without creating a
release (for example after changing the build or packaging scripts):
```bash
gh workflow run release.yml --ref dev
```

The packaged files are kept as workflow artifacts for 14 days.

## Building Locally (Fallback)

The workflow uses the same scripts as a local build, so a release can still be
built by hand if GitHub Actions is unavailable. Package with
`node scripts/package-release.mjs`, which empties `release-artifacts/` and fills
it with the correctly named files for the current platform, then upload with
`gh release upload vX.Y.Z release-artifacts/*`.

- **Linux** (needs `podman`): `npm run release:linux`. Builds inside the Ubuntu
  22.04 container (`src-tauri/packaging/linux/Containerfile`); a host
  `npm run tauri:build` is for local testing only and is never packaged. Linux
  binaries link against the glibc of the machine that builds them, and the
  AppImage also bundles that machine's libraries, so building on a recent
  distro would break older ones.
- **Windows:** `npm run tauri:build` from a checkout of the tag.
- **macOS** (Apple Silicon): `npm run tauri:build -- --bundles app`.

## Agent Checklist

- [ ] Version prepared with `npm run release:prepare`; tag is `vX.Y.Z`.
- [ ] Release workflow green: build, verify and install test on all three platforms.
- [ ] Notes come from `npm run release:notes`; no `TODO` remains; Downloads block untouched.
- [ ] Assets come only from the workflow (or `package-release.mjs`), never renamed by hand.
- [ ] `npm run release:promote` only when the maintainer has asked for this
      release to be published, and only after every check above passes.
