#!/bin/sh
# Nightly: full database copy to /backups, keep the newest 7.
set -eu
file="/backups/mailrust-$(date +%Y-%m-%d).dump"
pg_dump --format=custom --file="$file" "$DATABASE_URL"
ls -1t /backups/mailrust-*.dump | tail -n +8 | xargs -r rm --
echo "backup written: $file"
