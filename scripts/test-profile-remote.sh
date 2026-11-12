#!/usr/bin/env bash
# Run inside a disposable Linux container as root, with openssh-server and gnupg installed.
set -euo pipefail
test "$(id -u)" = 0
repo_root=$(cd "$(dirname "$0")/.." && pwd)
fixture=$(mktemp -d /tmp/gitpilot-remote.XXXXXX)
mkdir -p "$fixture/home/.ssh" "$fixture/client" "$fixture/gnupg" /run/sshd
chmod 700 "$fixture/gnupg"
: > "$fixture/empty.config"
for name in host key-a key-b; do
  ssh-keygen -q -t ed25519 -N '' -f "$fixture/$name"
done
cp "$fixture/key-a" "$fixture/home/.ssh/id_ed25519"
printf 'fixture@example.test %s\n' "$(cat "$fixture/key-b.pub")" > "$fixture/allowed_signers"
port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()')
printf '[127.0.0.1]:%s %s\n' "$port" "$(cat "$fixture/host.pub")" > "$fixture/known_hosts"
git -c init.defaultBranch=main init --bare --quiet "$fixture/remote.git"
cat > "$fixture/sshd_config" <<EOF
ListenAddress 127.0.0.1
Port $port
HostKey $fixture/host
PidFile $fixture/sshd.pid
AuthorizedKeysFile $fixture/key-a.pub
StrictModes no
PermitRootLogin prohibit-password
PasswordAuthentication no
KbdInteractiveAuthentication no
UsePAM no
AllowUsers root
EOF
/usr/sbin/sshd -D -e -f "$fixture/sshd_config" > "$fixture/sshd.log" 2>&1 &
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT
for attempt in {1..50}; do
  if python3 -c "import socket; s=socket.create_connection(('127.0.0.1',$port),.2); s.close()" 2>/dev/null; then break; fi
  sleep .1
done
GNUPGHOME="$fixture/gnupg" gpg --batch --pinentry-mode loopback --passphrase '' --quick-generate-key 'Fixture <fixture@example.test>' ed25519 sign 1d
fingerprint=$(GNUPGHOME="$fixture/gnupg" gpg --batch --with-colons --list-secret-keys | awk -F: '$1 == "fpr" { print $10; exit }')
cd "$repo_root/apps/desktop/src-tauri"
GITPILOT_REMOTE_TEST_ROOT="$fixture" GITPILOT_REMOTE_TEST_PORT="$port" GITPILOT_REMOTE_TEST_GPG_KEY="$fingerprint" \
  cargo test --locked --features custom-protocol profile_remote_keys_and_commit_signatures_are_independent -- --ignored --nocapture
