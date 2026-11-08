#!/usr/bin/env bash
set -euo pipefail
release="${HUSH_SETUP_RELEASE:-v0.5.3}"
if [[ ! "$release" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$ ]]; then echo 'Invalid release version.' >&2; exit 1; fi
tool_home="${HUSH_SETUP_HOME:-${XDG_DATA_HOME:-$HOME/.local/share}/hush/server-tools}"
mkdir -p "$tool_home"
scratch="$(mktemp -d "$tool_home/bootstrap.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
download() { curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 --connect-timeout 20 --max-time 300 "$1" -o "$2"; }
checksum() {
  local actual
  if command -v sha256sum >/dev/null 2>&1; then actual="$(sha256sum "$1" | awk '{print $1}')"
  else actual="$(shasum -a 256 "$1" | awk '{print $1}')"; fi
  if [[ "$actual" != "$2" ]]; then echo "Checksum verification failed: $1" >&2; exit 1; fi
}
echo 'Hush host setup. GitHub distributes the tools; Cloudflare runs the room server.'
node_bin=''
if [[ "${HUSH_USE_LOCAL_NODE:-0}" != 1 ]] && command -v node >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
  if [[ "$(node -p 'Number(process.versions.node.split(".")[0]) >= 22')" == true ]]; then node_bin="$(command -v node)"; fi
fi
if [[ -z "$node_bin" && -x "$tool_home/runtime/bin/node" ]]; then
  if [[ "$("$tool_home/runtime/bin/node" -p 'Number(process.versions.node.split(".")[0]) >= 22')" == true ]]; then node_bin="$tool_home/runtime/bin/node"; fi
fi
if [[ -z "$node_bin" ]]; then
  case "$(uname -s)" in Darwin) os='darwin'; suffix='tar.gz' ;; Linux) os='linux'; suffix='tar.xz' ;; *) echo 'Use the PowerShell script on Windows.' >&2; exit 1 ;; esac
  case "$(uname -m)" in arm64|aarch64) arch='arm64' ;; x86_64|amd64) arch='x64' ;; *) echo 'Supported architectures: x64 and ARM64.' >&2; exit 1 ;; esac
  echo 'Installing a private Node 24 runtime for host tools. No sudo is needed.'
  base='https://nodejs.org/dist/latest-v24.x'
  download "$base/SHASUMS256.txt" "$scratch/node-sha.txt"
  record="$(awk -v ending="-$os-$arch.$suffix" '$2 ~ /^node-v24\.[0-9]+\.[0-9]+-/ && substr($2,length($2)-length(ending)+1)==ending {print $1 " " $2}' "$scratch/node-sha.txt")"
  if [[ "$(printf '%s\n' "$record" | wc -l | tr -d ' ')" != 1 || ! "$record" =~ ^[a-f0-9]{64}\ node-v24\. ]]; then echo 'Could not locate the official Node archive.' >&2; exit 1; fi
  expected="${record%% *}"; filename="${record#* }"
  download "$base/$filename" "$scratch/node.$suffix"
  checksum "$scratch/node.$suffix" "$expected"
  mkdir -p "$scratch/runtime"
  tar -xf "$scratch/node.$suffix" --strip-components=1 -C "$scratch/runtime"
  if [[ -d "$tool_home/runtime" ]]; then rm -rf "$tool_home/runtime"; fi
  mv "$scratch/runtime" "$tool_home/runtime"
  node_bin="$tool_home/runtime/bin/node"
fi
if [[ "${HUSH_BOOTSTRAP_ONLY:-0}" == 1 ]]; then "$node_bin" --version; exit 0; fi
base="https://github.com/basilaslam/hush-native/releases/download/$release"
if [[ -n "${HUSH_SETUP_BUNDLE_DIR:-}" ]]; then
  cp "$HUSH_SETUP_BUNDLE_DIR/SHA256SUMS.txt" "$scratch/sums.txt"
  cp "$HUSH_SETUP_BUNDLE_DIR/Hush-Server-Setup.zip" "$scratch/server.zip"
else
  download "$base/SHA256SUMS.txt" "$scratch/sums.txt"
  download "$base/Hush-Server-Setup.zip" "$scratch/server.zip"
fi
expected="$(tr -d '\r' < "$scratch/sums.txt" | awk '$2=="Hush-Server-Setup.zip" {print $1}')"
if [[ ! "$expected" =~ ^[a-f0-9]{64}$ ]]; then echo 'Missing or ambiguous server setup checksum.' >&2; exit 1; fi
checksum "$scratch/server.zip" "$expected"
mkdir -p "$scratch/server"
if command -v unzip >/dev/null 2>&1; then unzip -q "$scratch/server.zip" -d "$scratch/server"
elif [[ "$(uname -s)" == Darwin ]]; then tar -xf "$scratch/server.zip" -C "$scratch/server"
else echo 'Install unzip, then rerun this script.' >&2; exit 1; fi
application="$tool_home/releases/$release"
mkdir -p "$application"
for name in setup.mjs worker.mjs package.json package-lock.json README.md; do cp "$scratch/server/$name" "$application/$name"; done
"$node_bin" "$application/setup.mjs" "$@"
