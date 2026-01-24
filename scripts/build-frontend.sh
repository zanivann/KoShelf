#!/usr/bin/env bash
set -e

echo "Building frontend assets..."

rm -rf assets_dist
mkdir -p assets_dist

npx tailwindcss -i assets/css/input.css -o assets_dist/style.css --minify
npx tailwindcss -i assets/css/calendar.css -o assets_dist/calendar_raw.css --minify

npx esbuild assets_dist/calendar_raw.css \
  --bundle --minify --loader:.css=css \
  --outfile=assets_dist/calendar.css

rm assets_dist/calendar_raw.css

npx esbuild \
  assets/ts/app/base.ts \
  assets/ts/pages/library_list.ts \
  assets/ts/pages/item_detail.ts \
  assets/ts/pages/statistics.ts \
  assets/ts/pages/recap.ts \
  assets/ts/pages/calendar.ts \
  assets/ts/app/service-worker.ts \
  --bundle \
  --format=esm \
  --target=es2020 \
  --minify \
  --entry-names=[name] \
  --outdir=assets_dist

echo "Frontend build complete."
