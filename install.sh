#!/usr/bin/env bash
# Builds code-seek from source and installs the binary into a prefix on the user's PATH.
# Usage: ./install.sh [--prefix <dir>]
set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly BINARY="code-seek"
# .cargo/config.toml redirects the build here, so the artifact is not under target/.
readonly BUILD_OUTPUT="${SCRIPT_DIR}/src/target/release/${BINARY}"
readonly DEFAULT_PREFIX="${HOME}/.local/bin"

_fail() {
	echo "install: $1" >&2
	exit 1
}

_usage() {
	cat <<-EOF >&2
	Usage: $(basename "$0") [--prefix <dir>]
	  --prefix  directory to install into (default: ${DEFAULT_PREFIX})
	EOF
	exit 1
}

# Warns instead of failing: the binary is installed either way, and the
# shell that would see the new PATH is not this one.
_check_path() {
	local prefix="$1"
	case ":${PATH}:" in
		*":${prefix}:"*) return 0 ;;
	esac
	echo "install: ${prefix} is not on your PATH — add it to your shell profile:"
	echo "  export PATH=\"${prefix}:\$PATH\""
}

main() {
	local prefix="${PREFIX:-${DEFAULT_PREFIX}}"

	while [[ $# -gt 0 ]]; do
		case "$1" in
			--prefix) shift; prefix="${1:-}" ;;
			--prefix=*) prefix="${1#--prefix=}" ;;
			-h|--help) _usage ;;
			*) _fail "unknown option: $1" ;;
		esac
		shift
	done

	[[ -n "${prefix}" ]] || _fail "--prefix is empty"
	command -v cargo >/dev/null 2>&1 \
		|| _fail "cargo not found — install the Rust toolchain from https://rustup.rs"

	echo "install: building ${BINARY}"
	cargo build --release --manifest-path "${SCRIPT_DIR}/Cargo.toml"
	[[ -f "${BUILD_OUTPUT}" ]] || _fail "the build produced no binary at ${BUILD_OUTPUT}"

	mkdir -p "${prefix}"
	install -m 755 "${BUILD_OUTPUT}" "${prefix}/${BINARY}"
	echo "install: ${BINARY} → ${prefix}/${BINARY}"

	_check_path "${prefix}"
	"${prefix}/${BINARY}" --version
}

main "$@"
