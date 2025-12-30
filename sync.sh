#!/bin/bash

set -ex

UPSTREAM_REPO="LocalizeLimbusCompany/LocalizeLimbusCompany"
NPM_PACKAGE="@lightsing/llc-zh-cn"

LATEST_RELEASE=$(curl -s https://api.github.com/repos/$UPSTREAM_REPO/releases/latest)
PUBLISHED_AT=$(echo "$LATEST_RELEASE" | jq -r '.published_at')
TAG=$(echo "$LATEST_RELEASE" | jq -r '.tag_name')
TIMESTAMP=$(node -e "console.log(Math.floor(new Date('$PUBLISHED_AT').getTime() / 1000))")
VERSION="1.0.$TIMESTAMP"

# Check current NPM version
LASTEST_NPM_VERSION=$(npm view "$NPM_PACKAGE" version)
LATEST_NPM_TIMESTAMP=$(echo "$LASTEST_NPM_VERSION" | awk -F. '{print $3}')

if [ "$TIMESTAMP" -le "$LATEST_NPM_TIMESTAMP" ]; then
    echo "No new release. Current NPM version: $LASTEST_NPM_VERSION, Latest upstream release timestamp: $TIMESTAMP"
    exit 0
fi

# Update package.json
jq ".githubTag = \"$TAG\" | .version = \"$VERSION\"" package.json > tmp.$$.json 
mv tmp.$$.json package.json

# If git reports no changes, exit
if git diff --quiet; then
    echo "No changes detected after updating package.json. Exiting."
    exit 0
fi

ZIP_LINK=$(echo "$LATEST_RELEASE" | jq -r '.assets[] | select(.name | endswith(".zip")) | .browser_download_url' | head -n 1)

curl -L -o release.zip "$ZIP_LINK"
unzip -o release.zip
rm release.zip

git config --global user.name "github-actions[bot]"
git config --global user.email "41898282+github-actions[bot]@users.noreply.github.com"
git add .
git commit -m "chore: sync to upstream release $TAG"
git push origin HEAD:llc-sync

npm publish --access public