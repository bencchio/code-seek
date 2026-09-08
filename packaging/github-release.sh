#!/usr/bin/env bash
# Publishes a GitHub release for an existing tag, attaching the Arch package built from that tag's
# own source — the source archives themselves come from GitHub (see README.md § Installation).
set -euo pipefail

TAG=""
DRY_RUN=0

_usage() {
	cat <<-EOF >&2
	Usage: $(basename "$0") <tag> [--dry-run]
	  <tag>      an existing tag to publish a release for, without the leading v (e.g. 0.6.0)
	  --dry-run  build and report, without touching GitHub or the repository
	EOF
	exit 1
}

_fail() {
	echo "github-release: $1" >&2
	exit 1
}

while [[ $# -gt 0 ]]; do
	case "$1" in
		--dry-run) DRY_RUN=1 ;;
		-h|--help) _usage ;;
		-*) echo "github-release: unknown option: $1" >&2; _usage ;;
		*) [[ -z "${TAG}" ]] || _fail "only one tag is accepted, got '${TAG}' and '$1'"; TAG="$1" ;;
	esac
	shift
done

[[ -n "${TAG}" ]] || _usage

# The published tags carry no prefix; the prefixed ones name private commits and must never be
# published, so accepting one here would attach a package built from a tree that cannot be served.
[[ "${TAG}" != v* ]] || _fail "public tags carry no leading v — pass '${TAG#v}' instead"

for tool in git curl makepkg gh; do
	command -v "${tool}" >/dev/null 2>&1 \
		|| _fail "missing required tool '${tool}' — install it first (e.g. pacman -S git curl base-devel github-cli on Arch)"
done

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly PKGBUILD="${SCRIPT_DIR}/aur/PKGBUILD"

[[ -f "${PKGBUILD}" ]] || _fail "no PKGBUILD at ${PKGBUILD}"

git -C "${REPO_ROOT}" rev-parse -q --verify "refs/tags/${TAG}" >/dev/null \
	|| _fail "tag '${TAG}' does not exist locally"

if ((DRY_RUN == 0)); then
	git -C "${REPO_ROOT}" ls-remote --exit-code --tags origin "refs/tags/${TAG}" >/dev/null 2>&1 \
		|| _fail "tag '${TAG}' is not on the remote — push it before publishing a release for it"
fi

workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT

# The checksum is taken from the archive GitHub serves rather than from the PKGBUILD, because a
# tarball cannot declare its own checksum: it contains the PKGBUILD that would have to state it.
readonly TARBALL_URL="$(git -C "${REPO_ROOT}" remote get-url origin | sed 's/\.git$//')/archive/refs/tags/${TAG}.tar.gz"
curl -fsSL "${TARBALL_URL}" -o "${workdir}/source.tar.gz" \
	|| _fail "could not download the source archive for '${TAG}' from ${TARBALL_URL}"
checksum="$(sha256sum "${workdir}/source.tar.gz" | cut -d' ' -f1)"

sed -e "s/^pkgver=.*/pkgver=${TAG}/" \
    -e "s/^sha256sums=.*/sha256sums=('${checksum}')/" \
    "${PKGBUILD}" > "${workdir}/PKGBUILD"

(cd "${workdir}" && makepkg >/dev/null)

package="$(find "${workdir}" -maxdepth 1 -type f -name '*.pkg.tar.zst' ! -name '*-debug-*' -print -quit)"
[[ -n "${package}" ]] || _fail "makepkg produced no package under ${workdir}"

notes="$(git -C "${REPO_ROOT}" tag -l --format='%(contents)' "${TAG}")
Architecture: $(uname -m)"

if ((DRY_RUN == 1)); then
	echo "github-release: would publish ${TAG} with:"
	echo "  $(basename "${package}")"
	echo "  plus the source archives GitHub generates for the tag"
	echo "--- release notes ---"
	echo "${notes}"
	exit 0
fi

readonly REMOTE="$(git -C "${REPO_ROOT}" remote get-url origin)"
if gh release view "${TAG}" --repo "${REMOTE}" >/dev/null 2>&1; then
	# Replaces the attachment in place, so a packaging fix can reach an existing release without
	# moving the tag it was cut from.
	gh release upload "${TAG}" "${package}" --repo "${REMOTE}" --clobber
else
	gh release create "${TAG}" \
		--repo "${REMOTE}" \
		--title "${TAG}" \
		--notes "${notes}" \
		"${package}"
fi

# Leaves the repository's PKGBUILD naming the release that now exists, so a clone builds the current
# one rather than whichever release it was last pinned to.
sed -i -e "s/^pkgver=.*/pkgver=${TAG}/" \
       -e "s/^sha256sums=.*/sha256sums=('${checksum}')/" \
       "${PKGBUILD}"

echo "github-release: published ${TAG}"
echo "github-release: PKGBUILD now points at ${TAG} — commit it"
