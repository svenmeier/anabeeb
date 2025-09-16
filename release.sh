#!/bin/bash
set -e

cargo deb

ARCH=$(uname -m)
VERSION=$(grep -m1 '^version' Cargo.toml | sed -E 's/version *= *"(.*)"/\1/')

DEB=$(find target/debian -name "*.deb" | head -n 1)
EXAMPLES="examples"
SCHEMAS="schemas"
JSON="*.json"
ZIP="target/anabeeb-${VERSION}-linux-${ARCH}.zip"

rm -f "$ZIP"
zip -j "$ZIP" $DEB
zip "$ZIP" $JSON
zip -r "$ZIP" $EXAMPLES
zip -r "$ZIP" $SCHEMAS

echo "✅ Linux release created: $ZIP"
