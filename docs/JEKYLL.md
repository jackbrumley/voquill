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
