# Installation and validation

## Install a platform wheel

Requires Python 3.10+. The stable release is `0.3.0`. Install from [PyPI](https://pypi.org/project/laymesh/):

```sh
python -m pip install laymesh
python -m laymesh --version
```

Alternatively, follow the [release procedure](../../release/README.en.md) to build a wheel, then install that exact `.whl` file with `python -m pip install /path/to/laymesh-<version>-<tags>.whl`. Use the wheel matching your OS, architecture and Linux glibc version. Repository examples are available separately in the [gallery](../sections/examples.en.md); they are not installed by pip.

A single installation provides the CLI, native plots, Python API, Jupyter magics, NumPy/pandas data bindings and Matplotlib Figure import. NumPy, pandas, Matplotlib and IPython are default dependencies.

Install in the same Python environment used by your Notebook kernel. In a Notebook, `%pip install laymesh` selects the active kernel's environment. A Notebook application such as JupyterLab is installed separately.

## Platforms and fonts

| Build target | Requirement |
| --- | --- |
| Windows x64 | 64-bit Windows, x64 Python |
| macOS Intel / Apple Silicon | macOS 14+, matching Python architecture |
| Linux x64 / arm64 | glibc at least as new as the wheel's `manylinux_2_XX` tag |

This release includes wheels for all five platforms, each verified on Python 3.10, 3.13 and 3.14. The Linux wheels are `manylinux_2_35_x86_64` / `manylinux_2_35_aarch64` and require glibc 2.35+. See the [release workflow](https://github.com/muxkin/LayMesh/actions/workflows/publish-pypi.yml) for validation. Alpine/musl, 32-bit systems and Windows ARM64 have no wheels.

Wheels contain one Rust native executable, formula fonts and dependency licenses. Rendering downloads no engine or fonts and requires no Rust, Node.js or TeX installation. Body fonts come from the system or user files; missing glyphs warn and display vector boxes. Supply explicit font files when sharing reproducible figures. Documentation browser previews provide their own fonts separately.

## First run

Save the following as `figure.lay`:

```lay
page = canvas(size=(120 mm, 90 mm), background="#ffffff")
p = plot(size=(110 mm, 80 mm),
         x=axis(label="Time (s)"), y=axis(label="Signal"))
p.line(x=[0, 1, 2, 3], y=[1, 3, 2, 4], label="Experiment")
p.legend(position="top_left")
page.add(p, offset=(5 mm, 5 mm))
```

```sh
python -m laymesh validate figure.lay
python -m laymesh inspect figure.lay --json
python -m laymesh render figure.lay -o figure.pdf
python -m laymesh render figure.lay -o figure.png --dpi 300
```

`laymesh` and `python -m laymesh` invoke the same engine. `validate` checks source and assets; `render` checks them and writes an export. Errors return a nonzero status with file, line, column and reason. `inspect --json` reports dimensions, mappings, transforms and diagnostics. Use `--warnings show|hide` or `LAYMESH_WARNINGS`; hiding warnings preserves errors and inspection diagnostics.

## Source development

Requires Rust 1.93.1 and Python 3.11+ for build scripts. From the repository root:

```sh
cargo build --release --locked -p laymesh-cli
python -m pip install -e './python'
python -m laymesh render examples/basic.lay -o basic.pdf
```

For direct native execution use `target/release/laymesh` (`target/release/laymesh.exe` on Windows), or `cargo run --release --locked -p laymesh-cli --`. Editable installs use the repository engine; they do not bundle it. `LAYMESH_CLI` can select an explicit native executable for development.

[Basic concepts](concepts.en.md) · [First layout](first-layout.en.md) · [Python API](python-reference.en.md) · [Notebook](notebook.en.md)
