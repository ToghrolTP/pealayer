#!/usr/bin/env bash
set -euo pipefail

check_only=false
if [[ "${1:-}" == "--check" ]]; then
  check_only=true
  shift
fi

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 [--check] /absolute/path/to/pealayer [output-directory]" >&2
  exit 2
fi

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ -f "$1" && -x "$1" ]] || {
  echo "screenshot executable does not exist or is not executable: $1" >&2
  exit 2
}
executable="$(realpath "$1")"
output_directory="${2:-$repository_root/docs/screenshots}"

for command in xdotool import sha256sum git; do
  command -v "$command" >/dev/null || {
    echo "required screenshot dependency is missing: $command" >&2
    exit 3
  }
done
[[ -n "${DISPLAY:-}" ]] || {
  echo 'DISPLAY is not set; run from the signed-in graphical session.' >&2
  exit 4
}

if $check_only; then
  echo "Linux screenshot preflight passed for $executable on DISPLAY=$DISPLAY"
  exit 0
fi

mkdir -p "$output_directory"
output_directory="$(realpath "$output_directory")"

source_commit="$(git -C "$repository_root" rev-parse HEAD)"
executable_hash="$(sha256sum "$executable" | awk '{print $1}')"
manifest_rows=()

for locale in en fa; do
  APP_NAME=Pealayer APP_THEME=dark APP_LOCALE="$locale" APP_DIRECTION=auto \
    "$executable" >/tmp/pealayer-screenshot-"$locale".log 2>&1 &
  process_id=$!
  cleanup() {
    if kill -0 "$process_id" 2>/dev/null; then
      kill "$process_id" 2>/dev/null || true
      wait "$process_id" 2>/dev/null || true
    fi
  }
  trap cleanup EXIT INT TERM

  window_id=""
  for _ in {1..150}; do
    window_id="$(xdotool search --onlyvisible --pid "$process_id" 2>/dev/null | head -n 1 || true)"
    [[ -n "$window_id" ]] && break
    kill -0 "$process_id" 2>/dev/null || {
      echo "Pealayer exited before its $locale window was ready." >&2
      exit 5
    }
    sleep 0.1
  done
  [[ -n "$window_id" ]] || {
    echo "Pealayer did not expose a window for locale $locale." >&2
    exit 6
  }

  xdotool windowsize "$window_id" 1280 800
  xdotool windowmove "$window_id" 32 32
  xdotool windowactivate --sync "$window_id"
  sleep 0.9
  file_name="pealayer-$locale-dark-linux.png"
  import -window "$window_id" "$output_directory/$file_name"
  capture_hash="$(sha256sum "$output_directory/$file_name" | awk '{print $1}')"
  direction=ltr
  [[ "$locale" == fa ]] && direction=rtl
  manifest_rows+=("    {\"file\":\"$file_name\",\"locale\":\"$locale\",\"direction\":\"$direction\",\"theme\":\"dark\",\"width\":1280,\"height\":800,\"sha256\":\"$capture_hash\"}")

  cleanup
  trap - EXIT INT TERM
done

{
  printf '{\n'
  printf '  "format": "pealayer-screenshots/v1",\n'
  printf '  "generated_at_utc": "%s",\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf '  "git_commit": "%s",\n' "$source_commit"
  printf '  "executable": "%s",\n' "$(basename "$executable")"
  printf '  "executable_sha256": "%s",\n' "$executable_hash"
  printf '  "captures": [\n'
  (IFS=$',\n'; printf '%s\n' "${manifest_rows[*]}")
  printf '  ]\n}\n'
} >"$output_directory/manifest-linux.json"

echo "Updated ${#manifest_rows[@]} Linux screenshots in $output_directory from $source_commit"
