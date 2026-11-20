## What and why

<!-- What does this pull request change, and why? Link the issue if there is one ("Fixes #12"). -->

## How it was tested

<!-- Demo car, a recording (--replay), or a real car: which car, which adapter? Screenshots help for the interface. -->

## Checks

The CI runs all of these. Tick what you ran yourself (see CONTRIBUTING.md, "What a pull request must pass").

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `python tools/i18n_rust.py --strict`
- [ ] `I18N_STRICT=1 npm --prefix app run build` and `I18N_STRICT=1 npm --prefix app test`
- [ ] `python tools/readme_languages.py --check` and `python docs/site/build.py --check`
- [ ] The generators leave nothing changed (`tools/gen_*.py`, `app/scripts/extract-catalog.py`), if data changed
- [ ] `python tools/dist_manifests.py --check-templates`, if something in `packaging/` changed

## House rules

- [ ] New texts are in German and English in the code, with translations for the other 16 languages (or I say below which are missing)
- [ ] No long dashes (U+2014, U+2013) in code, comments, texts or docs
- [ ] No code copied from GPL projects or code with an unclear licence; new data is freely licensed and names its source
- [ ] Nothing new writes to a car without a confirmation, and nothing codes, writes adaptation values, flashes or touches the immobiliser, keys or the mileage
- [ ] Generated files were not edited by hand
