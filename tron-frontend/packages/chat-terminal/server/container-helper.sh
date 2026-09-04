#!/usr/bin/env bash

# Run an explicitly requested interactive Bash/Zsh container shell through the
# same non-interactive bootstrap used by SSH. The user types only the original
# `docker exec -it CONTAINER bash` command; the generated bootstrap never
# reaches the container shell's readline history.
set -euo pipefail

runtime=${1-}
shift || true
original=("$@")

if [[ -z "$runtime" || ! -x "$runtime" || ${1-} != "exec" ]]; then
  exec "$runtime" "${original[@]}"
fi

integration=${__TRON_SSH_INTEGRATION-}
ssh_helper=${__TRON_SSH_HELPER-}
ssh_bootstrap=${__TRON_SSH_BOOTSTRAP-}
container_helper=${__TRON_CONTAINER_HELPER-}

encode_file() {
  local path=$1
  [[ -r "$path" ]] || return 1
  base64 <"$path" | tr -d '\r\n'
}

is_interactive_option() {
  local option=$1
  [[ "$option" == "-i" || "$option" == "--interactive" || "$option" == -*i* ]]
}

args=("$@")
prefix=(exec)
interactive=0
index=1

while (( index < ${#args[@]} )); do
  argument=${args[index]}
  case "$argument" in
    --)
      # We need to add -e variables before the container name. Preserve this
      # uncommon option terminator verbatim rather than moving new options
      # after it and changing Docker/Podman argument semantics.
      exec "$runtime" "${original[@]}"
      ;;
    --detach-keys|--env|--env-file|--user|--workdir|-e|-u|-w)
      (( index + 1 < ${#args[@]} )) || exec "$runtime" "${original[@]}"
      prefix+=("$argument" "${args[index + 1]}")
      ((index += 2))
      ;;
    --detach-keys=*|--env=*|--env-file=*|--user=*|--workdir=*)
      prefix+=("$argument")
      ((index += 1))
      ;;
    -*)
      is_interactive_option "$argument" && interactive=1
      prefix+=("$argument")
      ((index += 1))
      ;;
    *)
      break
      ;;
  esac
done

(( interactive == 1 && index < ${#args[@]} )) || exec "$runtime" "${original[@]}"
container=${args[index]}
((index += 1))
(( index + 1 == ${#args[@]} )) || exec "$runtime" "${original[@]}"
requested_shell=${args[index]}
case "${requested_shell##*/}" in
  bash|zsh) ;;
  *) exec "$runtime" "${original[@]}" ;;
esac

integration_base64=$(encode_file "$integration") || exec "$runtime" "${original[@]}"
helper_base64=$(encode_file "$ssh_helper") || exec "$runtime" "${original[@]}"
bootstrap_base64=$(encode_file "$ssh_bootstrap") || exec "$runtime" "${original[@]}"
container_helper_base64=$(encode_file "$container_helper") || exec "$runtime" "${original[@]}"

# The command runs under a fresh non-interactive /bin/sh. It restores the
# caller's actual startup files before exec'ing the requested interactive
# shell, so `history` remains owned by the container rather than Tron.
remote_bootstrap="eval \"\$(printf %s '$bootstrap_base64' | { base64 -d 2>/dev/null || base64 -D; })\""
exec "$runtime" "${prefix[@]}" \
  -e "__TRON_INTEGRATION_BASE64=$integration_base64" \
  -e "__TRON_SSH_HELPER_BASE64=$helper_base64" \
  -e "__TRON_SSH_BOOTSTRAP_BASE64=$bootstrap_base64" \
  -e "__TRON_CONTAINER_HELPER_BASE64=$container_helper_base64" \
  -e "__TRON_REMOTE_SHELL=$requested_shell" \
  "$container" /bin/sh -c "$remote_bootstrap"
