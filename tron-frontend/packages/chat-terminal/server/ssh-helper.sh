#!/bin/sh

real_ssh=${__TRON_REAL_SSH-}
bootstrap=${__TRON_SSH_BOOTSTRAP-}

if [ -z "$real_ssh" ] || [ ! -x "$real_ssh" ]; then
  printf '%s\n' 'tron: ssh executable is unavailable' >&2
  exit 127
fi

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

if [ "$target_seen" -ne 1 ] || [ -z "$bootstrap" ]; then
  interactive=0
fi

if [ "$interactive" -eq 1 ]; then
  exec "$real_ssh" "$@" -t "$bootstrap"
fi

exec "$real_ssh" "$@"
