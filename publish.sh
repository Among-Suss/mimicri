#! /bin/bash

# Possible arguments:
#   - patch (default)
#   - minor 
#   - major 

set -e

if [ "$1" == "" ] || [ "$1" == "patch" ]; then 
  bump_type="patch"
elif [ "$1" == "minor" ] || [ "$1" == "major" ]; then
  bump_type="$1"
else
  echo "Invalid bump type. Possible arguments: 'patch' (default), 'minor', or 'major'."
fi

# Check if dependencies install
jq --version &> /dev/null
if [ $? != 0 ]; then
  echo "Error: jq could not be found."
  exit
fi

cargo bump --version &> /dev/null
if [ $? != 0 ]; then
  echo "Error: cargo bump could not be found. Run 'cargo install cargo-bump'."
  exit
fi


previous_version=v$(cargo metadata --no-deps --format-version=1 | jq -r .packages[0].version)

cargo bump $bump_type
cargo check

version=v$(cargo metadata --no-deps --format-version=1 | jq -r .packages[0].version)

echo "Bumped version from $previous_version to $version"

git add .
git commit -m "Published $version" --allow-empty

git tag $version
git push origin main --tags