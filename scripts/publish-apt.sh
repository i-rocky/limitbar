#!/usr/bin/env bash
# Build and publish the Rocky OSS apt repository to Cloudflare R2.
#
# Pulls .deb assets from the latest GitHub release of each repo in REPOS,
# generates signed apt metadata (Packages, Release, InRelease), and uploads
# the tree to the R2 bucket via wrangler. Requirements: gh, dpkg-scanpackages
# (dpkg-dev), apt-ftparchive (apt-utils), gpg with the signing key below,
# and a wrangler login that can write to the bucket.
set -euo pipefail

REPOS=("i-rocky/limitbar")
BUCKET="rocky-apt"
SUITE="stable"
COMPONENT="main"
ARCHES=("amd64" "arm64")
KEY_ID="${ROCKY_APT_KEY:-Rocky OSS APT Repository}"

workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT
repo="${workdir}/repo"
mkdir -p "${repo}"

echo "==> downloading debs"
for gh_repo in "${REPOS[@]}"; do
  pkg="${gh_repo##*/}"
  dest="${repo}/pool/${COMPONENT}/${pkg:0:1}/${pkg}"
  mkdir -p "${dest}"
  gh release download --repo "${gh_repo}" --pattern '*.deb' --dir "${dest}"
done

cd "${repo}"

echo "==> generating Packages indices"
for arch in "${ARCHES[@]}"; do
  dir="dists/${SUITE}/${COMPONENT}/binary-${arch}"
  mkdir -p "${dir}"
  dpkg-scanpackages --arch "${arch}" pool /dev/null > "${dir}/Packages"
  gzip -9 -kf "${dir}/Packages"
done

echo "==> generating Release"
apt-ftparchive \
  -o "APT::FTPArchive::Release::Origin=Rocky OSS" \
  -o "APT::FTPArchive::Release::Label=Rocky OSS" \
  -o "APT::FTPArchive::Release::Suite=${SUITE}" \
  -o "APT::FTPArchive::Release::Codename=${SUITE}" \
  -o "APT::FTPArchive::Release::Components=${COMPONENT}" \
  -o "APT::FTPArchive::Release::Architectures=${ARCHES[*]}" \
  release "dists/${SUITE}" > "dists/${SUITE}/Release"

echo "==> signing"
gpg --batch --yes --local-user "${KEY_ID}" \
  --clearsign -o "dists/${SUITE}/InRelease" "dists/${SUITE}/Release"
gpg --batch --yes --local-user "${KEY_ID}" \
  --detach-sign --armor -o "dists/${SUITE}/Release.gpg" "dists/${SUITE}/Release"
gpg --batch --yes --export "${KEY_ID}" > rocky-oss.gpg
gpg --batch --yes --export --armor "${KEY_ID}" > rocky-oss.asc

echo "==> uploading to r2://${BUCKET}"
find . -type f | sed 's|^\./||' | while read -r key; do
  echo "  ${key}"
  npx --yes wrangler r2 object put "${BUCKET}/${key}" --file "${key}" --remote >/dev/null
done

echo "done."
