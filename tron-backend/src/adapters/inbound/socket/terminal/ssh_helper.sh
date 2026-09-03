#!/bin/sh

real_ssh=${__TRON_REAL_SSH-}
integration=${__TRON_SSH_INTEGRATION-}
helper=${__TRON_SSH_HELPER-}
bootstrap=${__TRON_SSH_BOOTSTRAP-}

if [ -z "$real_ssh" ] || [ ! -x "$real_ssh" ]; then
  printf '%s\n' 'tron: ssh executable is unavailable' >&2
  exit 127
fi

encode_file() {
  if [ ! -r "$1" ]; then
    return 1
  fi

  # macOS base64 wraps long output while GNU base64 usually does not. The remote
  # command is an assignment expression, so its payload must stay on one line.
  encoded=$(base64 <"$1" | tr -d '\r\n') || return 1
  [ -n "$encoded" ] || return 1
  printf %s "$encoded"
}

# Do not let an unsupported local locale reach the server through SendEnv.
unset LANG LC_ALL LC_CTYPE LC_MESSAGES LC_COLLATE LC_MONETARY LC_NUMERIC
unset LC_TIME LC_PAPER LC_NAME LC_ADDRESS LC_TELEPHONE LC_MEASUREMENT
unset LC_IDENTIFICATION

interactive=1
target_seen=0
option_value=0
options_done=0

for argument do
  if [ "$target_seen" -eq 1 ]; then
    interactive=0
    break
  fi

  if [ "$option_value" -eq 1 ]; then
    option_value=0
    continue
  fi

  if [ "$options_done" -eq 1 ]; then
    target_seen=1
    continue
  fi

  case "$argument" in
    --)
      options_done=1
      ;;
    -T|-N|-n|-f|-s|-G|-Q|-V|-W|-W?*|-O|-O?*)
      interactive=0
      ;;
    -B|-b|-c|-D|-E|-e|-F|-I|-i|-J|-L|-l|-m|-o|-P|-p|-R|-S|-w)
      option_value=1
      ;;
    -*)
      ;;
    *)
      target_seen=1
      ;;
  esac
done

if [ "$target_seen" -ne 1 ]; then
  interactive=0
fi

if [ "$interactive" -eq 1 ]; then
  integration_base64=$(encode_file "$integration") || {
    printf '%s\n' 'tron: SSH integration script is unavailable' >&2
    exit 1
  }
  helper_base64=$(encode_file "$helper") || {
    printf '%s\n' 'tron: SSH helper script is unavailable' >&2
    exit 1
  }
  bootstrap_base64=$(encode_file "$bootstrap") || {
    printf '%s\n' 'tron: SSH bootstrap script is unavailable' >&2
    exit 1
  }

  # Transfer the hook, helper and bootstrap template with every interactive
  # hop. The remote bootstrap restores them under its own 0700 directory, so
  # the sourced hook can wrap another `ssh` command without asking the browser
  # to guess the next remote prompt.
  remote_bootstrap="__TRON_INTEGRATION_BASE64=$integration_base64 __TRON_SSH_HELPER_BASE64=$helper_base64 __TRON_SSH_BOOTSTRAP_BASE64=$bootstrap_base64 eval \"\$(printf %s '$bootstrap_base64' | { base64 -d 2>/dev/null || base64 -D; })\""

  # SSH options must precede the destination. Keep the bootstrap as the
  # single remote command after all caller-supplied options and destination.
  exec "$real_ssh" -t "$@" "$remote_bootstrap"
fi

exec "$real_ssh" "$@"
