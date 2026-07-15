#!/bin/sh
set -eu

cd "$(dirname "$0")"
umask 077
mkdir -p secrets

printf 'Futu numeric user ID: ' >/dev/tty
IFS= read -r account </dev/tty
case "$account" in
  ''|*[!0-9]*)
    echo 'The user ID must contain digits only.' >&2
    exit 64
    ;;
esac

restore_tty() {
  stty echo </dev/tty 2>/dev/null || true
}
trap restore_tty EXIT HUP INT TERM
printf 'Futu login password (hidden): ' >/dev/tty
stty -echo </dev/tty
IFS= read -r password </dev/tty
restore_tty
trap - EXIT HUP INT TERM
printf '\n' >/dev/tty

if [ -z "$password" ]; then
  echo 'The password cannot be empty.' >&2
  exit 64
fi

password_md5="$(printf '%s' "$password" | md5sum | awk '{print $1}')"
unset password

printf '%s' "$account" > secrets/futu_account
printf '%s' "$password_md5" > secrets/futu_password_md5
chmod 600 secrets/futu_account secrets/futu_password_md5
chown 10001:10001 secrets/futu_account secrets/futu_password_md5
unset password_md5

echo 'OpenD secret files were written with mode 600 for the container user.'
