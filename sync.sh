#!/bin/bash

set -ex

UPSTREAM_REPO="LocalizeLimbusCompany/LocalizeLimbusCompany"

LATEST_RELEASE=$(curl -s https://api.github.com/repos/$UPSTREAM_REPO/releases/latest)
PUBLISHED_AT=$(echo "$LATEST_RELEASE" | jq -r '.published_at')

TAG=$(echo "$LATEST_RELEASE" | jq -r '.tag_name')
LAST_TAG=$(jq -r .githubTag package.json)

if [ "$TAG" == "$LAST_TAG" ]; then
    echo "No new release. Current tag: $LAST_TAG"
    exit 0
fi
jq ".githubTag = \"$TAG\"" package.json > tmp.$$.json 
mv tmp.$$.json package.json

ZIP_LINK=$(echo "$LATEST_RELEASE" | jq -r '.assets[] | select(.name | endswith(".zip")) | .browser_download_url' | head -n 1)

curl -L -o release.zip "$ZIP_LINK"
unzip -o release.zip
rm release.zip

TIMESTAMP=$(node -e "console.log(Math.floor(new Date('$PUBLISHED_AT').getTime() / 1000))")
jq ".version = \"1.0.$TIMESTAMP\"" package.json > tmp.$$.json 
mv tmp.$$.json package.json

git config --global user.name "github-actions[bot]"
git config --global user.email "41898282+github-actions[bot]@users.noreply.github.com"
git add .
git commit -m "chore: sync to upstream release $TAG"
git push origin HEAD:llc-sync

npm publish --access public