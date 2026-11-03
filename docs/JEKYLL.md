# Jekyll Local Preview

Jekyll is the static-site generator used by GitHub Pages. In this repo, the `docs/` folder is the site source, and Jekyll builds that content into `_site/` for local preview.

## Install

```bash
gem install jekyll bundler
```

## Run the local server

From the repo root:

```bash
jekyll serve --source docs --destination docs/_site
```

Then open `http://127.0.0.1:4000`.

## GitHub Pages relation

- GitHub Pages can run Jekyll automatically when publishing.
- Running Jekyll locally helps you preview `docs/` changes before pushing.
- The generated `docs/_site/` output is build output and should not be committed.

## Site structure

- `_layouts/default.html` wraps every page with the top bar (`_includes/site-bar.html`) and the footer (`_includes/site-footer.html`: support band, "Other apps" tiles, links). `_layouts/prose.html` is for Markdown guides (`INSTALL.md`, `UNINSTALL.md`).
- Each HTML page opens with a `<header class="hero">` (gradient band with the angled bottom edge and `_includes/hero-art.html`), followed by `<main class="page page-overlap">` so the first panel overlaps the hero edge.
- `assets/base.css` holds the shared colour tokens (light and dark via `prefers-color-scheme`) and components; each page adds its own stylesheet through the `stylesheets` front matter.
- Fonts are self-hosted in `assets/fonts/` (Inter and JetBrains Mono from Fontsource, SIL Open Font License; licence files alongside). The "Other apps" icons in `assets/icons/` are local copies from brumley.info. Do not add third-party font or asset requests.
