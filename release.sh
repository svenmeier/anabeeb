#!/bin/bash
set -e

cargo deb

ARCH=$(uname -m)
DEB=$(find target/debian -name "*.deb" | head -n 1)
EXAMPLES="examples"
SCHEMAS="schemas"
DISPOSITION="disposition.json"
ZIP="target/anabeeb-linux-$ARCH.zip"

rm -f "$ZIP"
zip -j "$ZIP" $DEB
zip "$ZIP" $DISPOSITION
zip -r "$ZIP" $EXAMPLES
zip -r "$ZIP" $SCHEMAS

echo "✅ Linux release created: $ZIP"
