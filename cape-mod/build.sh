#!/usr/bin/env bash
# Builds the Agate Cape mod against an installed Minecraft 26.x + Fabric (no Gradle needed:
# 26.x is not obfuscated, so the names used here are the names the game runs with).
# Needs: a JDK 25 in launcher/.tools, jspecify (compile-only annotations) in .tools,
# and Minecraft 26.3 + Fabric installed by Agate.
# Usage: cape-mod/build.sh [Agate data folder]   -> src-tauri/assets/agate-cape.jar
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
DATA="${1:-$APPDATA/Agate}"
[ -d "$DATA" ] || DATA="$APPDATA/Blocco"   # data not migrated yet (folder of the old name)
JDK="$(ls -d "$HERE"/../.tools/jdk-25* | head -1)/bin"
TOOLS="$(cygpath -w "$HERE/../.tools")"
CP="$(python - "$DATA" "$TOOLS" <<'PY'
import glob, json, os, sys
data, tools = sys.argv[1], sys.argv[2]
v = json.load(open(os.path.join(data, "versions", "26.3", "26.3.json")))
cp = [os.path.join(data, "versions", "26.3", "26.3.jar")]
for lib in v["libraries"]:
    a = lib.get("downloads", {}).get("artifact")
    if a:
        cp.append(os.path.join(data, "libraries", *a["path"].split("/")))
for pattern in ("net/fabricmc/fabric-loader/*/fabric-loader-*.jar", "net/fabricmc/sponge-mixin/*/sponge-mixin-*.jar"):
    cp.append(sorted(glob.glob(os.path.join(data, "libraries", *pattern.split("/"))))[-1])
cp.append(os.path.join(tools, "jspecify-1.0.0.jar"))
print(";".join(p for p in cp if os.path.exists(p)), end="")
PY
)"
OUT="$HERE/build"
rm -rf "$OUT" && mkdir -p "$OUT/classes"
"$JDK/javac" --release 21 -nowarn -cp "$CP" -d "$OUT/classes" $(find "$HERE/src" -name '*.java')
cp "$HERE"/resources/* "$OUT/classes/"
mkdir -p "$HERE/../src-tauri/assets"
rm -f "$HERE/../src-tauri/assets/agate-cape.jar"
(cd "$OUT/classes" && "$JDK/jar" --create --file "$HERE/../src-tauri/assets/agate-cape.jar" .)
echo "built $(ls -la "$HERE/../src-tauri/assets/agate-cape.jar")"
