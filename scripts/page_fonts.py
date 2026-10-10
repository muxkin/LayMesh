"""Deterministic Page-only startup fonts, with full fonts retained for editing."""
from io import BytesIO
import hashlib
import json
from pathlib import Path

import fontTools
from fontTools import subset
from fontTools.ttLib import TTCollection

from site_support import ROOT, dependencies, examples

FONTTOOLS_VERSION = '4.62.1'
FONTS = [('DejaVu Sans', 'DejaVuSans.ttf', 'DejaVu-LICENSE'),
         ('Noto Sans CJK SC', 'NotoSansCJK-Regular.ttc', 'Noto-CJK-LICENSE')]


def startup_characters():
    paths = {p for e in examples() for p in dependencies(e['source'])
             if Path(p).suffix.lower() in ('.lay', '.laydoc', '.lcss', '.css', '.csv', '.json')}
    chars = set(range(32, 256))
    for path in paths:
        chars.update(map(ord, (ROOT / path).read_text()))
    return sorted(chars)


def assets():
    assert fontTools.__version__ == FONTTOOLS_VERSION, 'Install release/docs-requirements.txt'
    chars = startup_characters()
    files, records = {}, []
    def asset(data, suffix):
        digest = hashlib.sha256(data).hexdigest()
        name = f'fonts/{digest}{suffix}'
        files[name] = data
        return {'file': name, 'sha256': digest, 'bytes': len(data)}
    for family, filename, license_name in FONTS:
        source = ROOT / 'site/page-fonts' / filename
        data = source.read_bytes()
        record = {'family': family, 'filename': filename}
        if source.suffix == '.ttc':
            collection = TTCollection(source, lazy=True)
            options = subset.Options()
            options.layout_features = ['*']
            options.recalc_timestamp = False
            options.name_IDs = ['*']
            options.name_languages = ['*']
            options.name_legacy = True
            identities, coverage = [], []
            for font in collection.fonts:
                font.recalcTimestamp = False
                names = [font['name'].getDebugName(n) for n in (1, 2, 6)]
                expected = set(chars) & set(font.getBestCmap())
                sub = subset.Subsetter(options=options)
                sub.populate(unicodes=chars)
                sub.subset(font)
                assert expected <= set(font.getBestCmap()), 'Startup font lost requested characters'
                assert names == [font['name'].getDebugName(n) for n in (1, 2, 6)]
                identities.append(names)
                coverage.append(sorted(expected))
            output = BytesIO()
            collection.save(output)
            collection.close()
            record.update(asset(output.getvalue(), '.ttc'))
            record['subset'] = {'charactersSha256': hashlib.sha256(json.dumps(chars).encode()).hexdigest(),
                                'faces': identities, 'coverage': coverage}
            record['full'] = asset(data, '.ttc')
        else:
            record.update(asset(data, source.suffix))
        records.append(record)
        files['fonts/' + license_name] = (source.parent / license_name).read_bytes()
    return records, files


def build(output):
    records, files = assets()
    for name, data in files.items():
        target = output / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    return records


def validate(output, manifest):
    records, files = assets()
    assert manifest['fonts'] == records, 'Stale startup font metadata; rebuild docs'
    for name, data in files.items():
        assert (output / name).read_bytes() == data, f'Stale Page font or license: {name}'
    for path in output.rglob('*'):
        if path.suffix.lower() in ('.ttf', '.otf', '.ttc', '.otc', '.woff', '.woff2'):
            assert path.relative_to(output).as_posix() in files, f'Unexpected Page font: {path}'
