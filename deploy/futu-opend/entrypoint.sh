#!/bin/sh
set -eu

account_file="${FUTU_ACCOUNT_FILE:-/run/secrets/futu_account}"
password_file="${FUTU_PASSWORD_MD5_FILE:-/run/secrets/futu_password_md5}"

if [ ! -r "$account_file" ] || [ ! -r "$password_file" ]; then
  echo "OpenD secret files are missing. See README.md for setup." >&2
  exit 64
fi

account="$(tr -d '\r\n' < "$account_file")"
password_md5="$(tr -d '\r\n' < "$password_file" | tr 'A-F' 'a-f')"

case "$account" in
  ''|*[!0-9]*)
    echo "futu_account must contain the numeric Futu user ID." >&2
    exit 64
    ;;
esac

case "$password_md5" in
  *[!0-9a-f]*|'')
    echo "futu_password_md5 must contain a 32-character lowercase hexadecimal MD5 value." >&2
    exit 64
    ;;
esac

if [ "${#password_md5}" -ne 32 ]; then
  echo "futu_password_md5 must contain exactly 32 hexadecimal characters." >&2
  exit 64
fi

case "${FUTU_API_PORT:-}" in
  ''|*[!0-9]*)
    echo "FUTU_API_PORT must be a numeric TCP port." >&2
    exit 64
    ;;
esac

case "${FUTU_AUTO_HOLD_QUOTE_RIGHT:-0}" in
  0|1) ;;
  *)
    echo "FUTU_AUTO_HOLD_QUOTE_RIGHT must be 0 or 1." >&2
    exit 64
    ;;
esac

config=/run/futu-opend/FutuOpenD.xml
cat > "$config" <<EOF
<futu_opend>
  <ip>0.0.0.0</ip>
  <api_port>${FUTU_API_PORT}</api_port>
  <login_account>${account}</login_account>
  <login_pwd_md5>${password_md5}</login_pwd_md5>
  <lang>chs</lang>
  <log_level>${FUTU_LOG_LEVEL}</log_level>
  <log_path>/var/lib/futu-opend/logs</log_path>
  <push_proto_type>0</push_proto_type>
  <price_reminder_push>0</price_reminder_push>
  <auto_hold_quote_right>${FUTU_AUTO_HOLD_QUOTE_RIGHT}</auto_hold_quote_right>
</futu_opend>
EOF
chmod 0600 "$config"
mkdir -p /var/lib/futu-opend/logs

exec /opt/futu-opend/FutuOpenD -cfg_file="$config"
