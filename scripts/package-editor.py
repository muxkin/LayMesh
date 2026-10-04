"""Package the built native extension for local installation or Marketplace upload.

Only Python's standard library is used. This command never publishes anything.
"""
import argparse
import json
import mimetypes
import re
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

TARGETS = ['win32-x64', 'win32-arm64', 'linux-x64', 'linux-arm64', 'linux-armhf',
           'alpine-x64', 'alpine-arm64', 'darwin-x64', 'darwin-arm64']
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', required=True, type=Path)
parser.add_argument('--extension-dir', type=Path, help='Package an isolated extension staging directory')
parser.add_argument('--target', choices=TARGETS)
parser.add_argument('--pre-release', action='store_true', help='Mark a numeric extension version as a Marketplace prerelease')
args = parser.parse_args()
root = args.extension_dir.resolve() if args.extension_dir else Path(__file__).resolve().parents[1] / 'extensions/vscode'
package = json.loads((root / 'package.json').read_text())
translations = json.loads((root / 'package.nls.json').read_text())

def localized(value):
    return translations[value[1:-1]] if value.startswith('%') and value.endswith('%') else value

if args.pre_release and not re.fullmatch(r'(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)', package['version']):
    parser.error('Marketplace prereleases require a numeric major.minor.patch version.')
for name in ['client.cjs', 'preview-host.cjs', 'preview-webview.mjs', 'preview-math.mjs', 'preview.css']:
    if not (root / 'dist' / name).is_file():
        raise SystemExit('Run python scripts/build-editors.py first.')

ns = 'http://schemas.microsoft.com/developer/vsx-schema/2011'
ET.register_namespace('', ns)
manifest = ET.Element(f'{{{ns}}}PackageManifest', {'Version': '2.0.0'})
def child(parent, tag, attrs=None, text=None):
    element = ET.SubElement(parent, f'{{{ns}}}{tag}', attrs or {})
    if text is not None:
        element.text = text
    return element

metadata = child(manifest, 'Metadata')
identity = {'Language': 'en-US', 'Id': package['name'], 'Version': package['version'], 'Publisher': package['publisher']}
if args.target:
    identity['TargetPlatform'] = args.target
child(metadata, 'Identity', identity)
child(metadata, 'DisplayName', text=localized(package['displayName']))
child(metadata, 'Description', {'{http://www.w3.org/XML/1998/namespace}space': 'preserve'}, localized(package['description']))
child(metadata, 'Tags', text=','.join(package.get('keywords', ['LayMesh', 'LCSS'])))
child(metadata, 'Categories', text=','.join(package.get('categories', [])))
child(metadata, 'GalleryFlags', text='Public Preview' if package.get('preview') else 'Public')
properties = child(metadata, 'Properties')
values = {
    'Microsoft.VisualStudio.Code.Engine': package['engines']['vscode'],
    'Microsoft.VisualStudio.Code.ExtensionDependencies': ','.join(package.get('extensionDependencies', [])),
    'Microsoft.VisualStudio.Code.ExtensionPack': ','.join(package.get('extensionPack', [])),
    'Microsoft.VisualStudio.Code.ExtensionKind': ','.join(package.get('extensionKind', ['workspace'])),
    'Microsoft.VisualStudio.Code.LocalizedLanguages': '',
    'Microsoft.VisualStudio.Code.EnabledApiProposals': ','.join(package.get('enabledApiProposals', [])),
    'Microsoft.VisualStudio.Code.ExecutesCode': 'true',
    'Microsoft.VisualStudio.Services.GitHubFlavoredMarkdown': 'true',
    'Microsoft.VisualStudio.Services.Content.Pricing': package.get('pricing', 'Free'),
}
if args.pre_release:
    values['Microsoft.VisualStudio.Code.PreRelease'] = 'true'
repository = package.get('repository', {}).get('url', '').removesuffix('.git')
if repository:
    values['Microsoft.VisualStudio.Services.Links.Source'] = repository
    values['Microsoft.VisualStudio.Services.Links.Getstarted'] = repository
    values['Microsoft.VisualStudio.Services.Links.GitHub' if repository.startswith('https://github.com/') else 'Microsoft.VisualStudio.Services.Links.Repository'] = repository
if package.get('bugs', {}).get('url'):
    values['Microsoft.VisualStudio.Services.Links.Support'] = package['bugs']['url']
if package.get('homepage'):
    values['Microsoft.VisualStudio.Services.Links.Learn'] = package['homepage']
for key, value in values.items():
    child(properties, 'Property', {'Id': key, 'Value': value})
# Match VSCE: extensionless LICENSE must become LICENSE.txt so the gallery
# recognizes it as a package part with a content type.
child(metadata, 'License', text='extension/LICENSE.txt')
if package.get('icon'):
    if not (root / package['icon']).is_file():
        raise SystemExit('Missing extension icon.')
    child(metadata, 'Icon', text='extension/' + package['icon'])
child(child(manifest, 'Installation'), 'InstallationTarget', {'Id': 'Microsoft.VisualStudio.Code'})
child(manifest, 'Dependencies')
assets = child(manifest, 'Assets')
asset_files = {
    'Microsoft.VisualStudio.Code.Manifest': 'package.json',
    'Microsoft.VisualStudio.Services.Content.Details': 'README.md',
    'Microsoft.VisualStudio.Services.Content.License': 'LICENSE.txt',
}
if (root / 'CHANGELOG.md').is_file():
    asset_files['Microsoft.VisualStudio.Services.Content.Changelog'] = 'CHANGELOG.md'
if package.get('icon'):
    asset_files['Microsoft.VisualStudio.Services.Icons.Default'] = package['icon']
for kind, file in asset_files.items():
    child(assets, 'Asset', {'Type': kind, 'Path': 'extension/' + file, 'Addressable': 'true'})

files = [file for file in sorted(root.rglob('*')) if file.is_file()
         and not {'src', 'node_modules', '__pycache__'}.intersection(file.relative_to(root).parts)
         and file.suffix.lower() not in ('.vsix', '.pyc', '.ttf', '.otf', '.ttc', '.otc', '.woff', '.woff2')]
if not (root / 'LICENSE').is_file():
    raise SystemExit('Missing extension LICENSE.')
def archive_path(file):
    relative = file.relative_to(root).as_posix()
    return 'extension/' + ('LICENSE.txt' if relative == 'LICENSE' else relative)

files = [file for file in files if file.relative_to(root).as_posix() != 'LICENSE.txt']
types = {'json': 'application/json', 'vsixmanifest': 'text/xml', 'cjs': 'application/javascript',
         'mjs': 'application/javascript', 'md': 'text/markdown', 'txt': 'text/plain'}
for file in files:
    suffix = Path(archive_path(file)).suffix
    if suffix:
        extension = suffix[1:].lower()
        types[extension] = types.get(extension, mimetypes.guess_type(file.name)[0] or 'application/octet-stream')
content = ET.Element('Types', {'xmlns': 'http://schemas.openxmlformats.org/package/2006/content-types'})
for extension, mime in sorted(types.items()):
    ET.SubElement(content, 'Default', {'Extension': '.' + extension, 'ContentType': mime})
for file in files:
    if not Path(archive_path(file)).suffix:
        ET.SubElement(content, 'Override', {'PartName': '/' + archive_path(file), 'ContentType': 'application/octet-stream'})

args.output.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(args.output, 'w', zipfile.ZIP_DEFLATED) as archive:
    archive.writestr('extension.vsixmanifest', ET.tostring(manifest, encoding='utf-8', xml_declaration=True))
    archive.writestr('[Content_Types].xml', ET.tostring(content, encoding='utf-8', xml_declaration=True))
    for file in files:
        archive.write(file, archive_path(file))
print(f'VSIX created: {args.output} (target={args.target or "local"}, prerelease={args.pre_release}); no publication performed.')
