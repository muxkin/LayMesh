# Build and publish to PyPI

Release **0.3.2** is being prepared with preview performance improvements, image hash reuse and cached font fallback rankings. VS Code 0.3.7 adds the configurable debounce command. Verify publication in the PyPI index.

The current release is **[`0.3.1`](https://pypi.org/project/laymesh/0.3.1/)** (all five platform wheels are published), shared by the Python package and Rust engine. NumPy, pandas, Matplotlib and IPython are default dependencies; `pip install laymesh` installs all Python features. The [publishing workflow](https://github.com/muxkin/LayMesh/actions/workflows/publish-pypi.yml) builds and verifies five platform wheels, audits their contents and checksums, and uploads through Trusted Publishing. Each wheel includes the Python API, CLI entry point, one native Rust executable, dependency licenses and a build manifest. Formula fonts are compiled into RaTeX; body fonts come from the system or user files.

[runtime.json](runtime.json) defines five targets: Linux x64 / arm64, macOS 14+ Intel / Apple Silicon, and Windows x64. Python users need 3.10+; build scripts need Python 3.11+ and Rust 1.93.1. This release publishes platform wheels only. The `python/` directory alone is not a complete source distribution with the Rust engine; do not upload an sdist made from that directory.

## Local build and audit

Use a virtual environment, install the pinned tooling, and run from the repository root:

```sh
python -m pip install -r release/build-requirements.txt
python scripts/check-release.py
python -m pip install -e "./python" pillow
python scripts/test-contracts.py
python scripts/collect-licenses.py
python scripts/build-python-wheel.py --output release/dist/pypi-0.3.1
python scripts/check-python-wheels.py release/dist/pypi-0.3.1 --checksums release/dist/pypi-0.3.1/SHA256SUMS
python -m twine check --strict release/dist/pypi-0.3.1/*.whl
```

The builder checks version alignment, compiles the locked release engine, and stages a wheel outside the source tree. A separate output directory avoids mixing this candidate with earlier wheels. Linux tags reflect the binary's measured GLIBC symbol requirements, with a floor of 2.28; a newer host build must not claim an older baseline. The local Linux x64 wheel requires glibc 2.35+. CI runner builds may require a newer glibc; review the actual filenames and manifests.

The manifest records Python/Rust versions, target, executable SHA-256 and Cargo.lock SHA-256. The audit checks metadata, platform tags, executable permissions, hashes, license texts and absence of local home paths. It rejects Node/npm, JS runtime files and body-font bundles. Recollect licenses whenever locked dependencies change; CI does this for every build.

## Verify the installed wheel

Create a **second, clean virtual environment** and install the generated wheel, not an editable checkout. Replace the example filename below with the actual wheel for your machine:

```sh
python -m pip install "release/dist/pypi-0.3.1/laymesh-0.3.1-py3-none-manylinux_2_35_x86_64.whl" pillow
python -m pip check
python -m laymesh --version
python scripts/smoke-python-wheel.py --examples examples
python -m unittest discover -s python/tests -v
```

The smoke test changes into a temporary directory and empties PATH. It checks SVG/PDF/PNG/JPEG/TIFF/WebP exports, CLI and module entry points, saved data, Matplotlib import, both Notebook magics and standalone example entry points using the bundled engine. Local checks certify only the tested platform. The workflow builds five platforms on Python 3.13 and installs the resulting wheels on Python 3.10 and 3.14 as well.

## GitHub and PyPI setup

The existing [publish workflow](../.github/workflows/publish-pypi.yml) uses [PyPA's Trusted Publishing procedure](https://packaging.python.org/en/latest/guides/publishing-package-distribution-releases-using-github-actions-ci-cd-workflows/). Before the first upload:

1. Configure a PyPI pending trusted publisher with project `laymesh`, owner `muxkin`, repository `LayMesh`, workflow `publish-pypi.yml`, and environment `pypi`.
2. Configure the repository's GitHub environment `pypi` with required reviewers. The workflow checks that this approval gate exists.
3. Make the final changes available on `main`. Run **Python wheels and PyPI** manually with `publish=false` to verify the candidate. Review all five wheels and checksums in `reviewed-python-release`.
4. For the actual upload, run the same workflow on `main` with `publish=true` and `version=0.3.1`. It builds and tests again, verifies the requested version and checksums, then waits for the configured environment approval. Approve the artifacts from that run.

The publishing job downloads the exact reviewed artifacts and uploads them without rebuilding. Pull requests and ordinary build runs do not upload. After publication, verify an index installation outside the checkout with `python -m pip install "laymesh==0.3.1"`, then `python -m pip check` and `python -m laymesh --version`. A published version cannot be overwritten; increment the Python/Rust/runtime versions together for a replacement release.

## Verification records

`python scripts/test-contracts.py` produces `release/verification/cargo-tests.log`, `python-tests.log` and `assertion-coverage.json`, checking current source identities, test execution and historical assertion mappings. Install the Poppler and MuPDF command-line tools before running these checks. Use the GitHub Actions runs for the current commit as cross-platform release evidence; the final wheels and checksums are stored in the `reviewed-python-release` artifact.

## Other artifacts

Build the native VS Code extension with `python scripts/build-editors.py`, package it with `python scripts/package-editor.py --output release/dist/laymesh.vsix`, and audit with `python scripts/check-editor.py release/dist/laymesh.vsix`. Build the Rust/WASM documentation site with `python scripts/build-docs.py`. These artifacts are distributed separately from the PyPI wheel.
