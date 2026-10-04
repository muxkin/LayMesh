"""Package the already built local extension without contacting a marketplace."""
import argparse,json
from pathlib import Path
import zipfile
from xml.sax.saxutils import escape
parser=argparse.ArgumentParser()
parser.add_argument('--output',required=True)
args=parser.parse_args()
root=Path(__file__).resolve().parents[1]/'extensions/vscode'
package=json.loads((root/'package.json').read_text())
for name in ['client']:
 if not (root/f'dist/{name}.cjs').is_file():raise SystemExit('Run python scripts/build-editors.py first.')
manifest=f'''<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011"><Metadata><Identity Language="en-US" Id="{package['name']}" Version="{package['version']}" Publisher="{package['publisher']}"/><DisplayName>{escape(package['displayName'])}</DisplayName><Description xml:space="preserve">{escape(package['description'])}</Description><Tags>LayMesh,LCSS</Tags><Categories>Programming Languages</Categories><Properties><Property Id="Microsoft.VisualStudio.Code.Engine" Value="{escape(package['engines']['vscode'])}"/><Property Id="Microsoft.VisualStudio.Code.ExtensionDependencies" Value=""/><Property Id="Microsoft.VisualStudio.Code.ExtensionPack" Value=""/></Properties></Metadata><Installation><InstallationTarget Id="Microsoft.VisualStudio.Code"/></Installation><Dependencies/><Assets><Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true"/><Asset Type="Microsoft.VisualStudio.Services.Content.Details" Path="extension/README.md" Addressable="true"/><Asset Type="Microsoft.VisualStudio.Services.Content.License" Path="extension/LICENSE" Addressable="true"/></Assets></PackageManifest>'''
output=Path(args.output);output.parent.mkdir(parents=True,exist_ok=True)
with zipfile.ZipFile(output,'w',zipfile.ZIP_DEFLATED) as archive:
 archive.writestr('extension.vsixmanifest',manifest)
 archive.writestr('[Content_Types].xml','<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="json" ContentType="application/json"/><Default Extension="cjs" ContentType="application/javascript"/><Default Extension="mjs" ContentType="application/javascript"/><Default Extension="md" ContentType="text/markdown"/><Default Extension="vsixmanifest" ContentType="text/xml"/><Default Extension="" ContentType="text/plain"/></Types>')
 for file in sorted(root.rglob('*')):
  if file.is_file() and 'src' not in file.relative_to(root).parts and 'node_modules' not in file.relative_to(root).parts and file.suffix.lower() not in ('.ttf','.otf','.ttc','.otc','.woff','.woff2'):archive.write(file,'extension/'+file.relative_to(root).as_posix())
print('Local VSIX package created; no publication performed.')
