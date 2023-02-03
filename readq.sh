#!/bin/busybox ash
set -euo pipefail

curl -v "https://synapse.matrix.msrd0.de/_matrix/client/v3/user/@tg2mx_bot:msrd0.de/account_data/de.msrd0.tg2mx_bot.queue?access_token=$(cat .env |grep ACCESS_TOKEN | tr '=' ' ' | awk '{print $2}')" |jq
