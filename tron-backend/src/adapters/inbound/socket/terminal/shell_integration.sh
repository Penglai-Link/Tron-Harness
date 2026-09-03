# Tron terminal shell integration.
#
# Reports semantic prompt boundaries so the client can split the raw PTY stream
# into command blocks:
#
#   OSC 133 ; A ST          prompt start
#   OSC 133 ; B ST          prompt end / command input starts
#   OSC 133 ; C ST          command output starts
#   OSC 133 ; D ; <exit> ST command finished
#   OSC 633 ; E ; <line> ST command line that is about to run
#   OSC 7  ; file://<host><cwd> ST  working directory
#
# The same script is sourced by the locally spawned shell and injected into
# remote shells over SSH, so it must detect the shell flavour at runtime and
# must never assume it is the outermost shell.

# Readline must see a locale that actually exists on the current machine.
# Keep LC_ALL unset so one unavailable local value cannot poison every locale
# category on a remote host.
unset LC_ALL
export LANG=C
unset LC_CTYPE
__tron_utf8_locale=
for __tron_locale_name in $(LC_ALL=C locale -a 2>/dev/null); do
  case "$__tron_locale_name" in
    *[Uu][Tt][Ff]-8*|*[Uu][Tt][Ff]8*)
      __tron_utf8_locale=$__tron_locale_name
      break
      ;;
  esac
done
if [ -n "$__tron_utf8_locale" ]; then
  export LC_CTYPE="$__tron_utf8_locale"
fi
unset __tron_locale_name __tron_utf8_locale

if [ -n "${ZSH_VERSION-}" ]; then

  # Every `;` before a closing `}` matters: `eval` parses this whole file in one
  # go, so the zsh branch must still be *parseable* by bash. Without the `;` bash
  # reads `}` as an argument, never closes the group, and dies on the `elif`
  # below — which is what broke the SSH bootstrap on every remote bash.
  __tron_osc() { printf '\033]%s\007' "$1"; }

  __tron_escape() {
    local value=$1
    value=${value//'\'/'\\'}
    value=${value//;/'\x3b'}
    printf '%s' "$value"
  }

  __tron_report_cwd() { __tron_osc "7;file://${HOST-}${PWD}"; }

  __tron_preexec() {
    __tron_executed=1
    __tron_osc "633;E;$(__tron_escape "$1")"
    __tron_osc "133;C"
  }

  __tron_precmd() {
    # Not `status`: in zsh that name is a read-only alias for `$?`, and assigning
    # to it aborts the hook before any marker is reported.
    local __tron_status=$?
    if [ -n "${__tron_executed-}" ]; then
      __tron_osc "133;D;$__tron_status"
      unset __tron_executed
    fi
    __tron_report_cwd
  }

  autoload -Uz add-zsh-hook 2>/dev/null
  if whence add-zsh-hook >/dev/null 2>&1; then
    add-zsh-hook precmd __tron_precmd
    add-zsh-hook preexec __tron_preexec
  else
    precmd_functions+=(__tron_precmd)
    preexec_functions+=(__tron_preexec)
  fi

  if [ -z "${__TRON_PROMPT_WRAPPED-}" ]; then
    __TRON_PROMPT_WRAPPED=1
    __tron_prompt_sync_end=
    if [ -n "${__TRON_SYNCED_BOOTSTRAP-}" ]; then
      __tron_prompt_sync_end='%{\e[?2026l%}'
      unset __TRON_SYNCED_BOOTSTRAP
    fi
    PS1="%{$(__tron_osc '133;A')%}${PS1}%{$(__tron_osc '133;B')%}${__tron_prompt_sync_end}"
    unset __tron_prompt_sync_end
  fi

  __tron_report_cwd

elif [ -n "${BASH_VERSION-}" ]; then

  __tron_osc() { printf '\033]%s\007' "$1"; }

  __tron_escape() {
    local value="$1"
    value="${value//\\/\\\\}"
    value="${value//;/\\x3b}"
    printf '%s' "$value"
  }

  __tron_report_cwd() { __tron_osc "7;file://${HOSTNAME-}${PWD}"; }

  # bash has no preexec, so recover the submitted line from history instead of
  # $BASH_COMMAND, which only holds the first component of a pipeline.
  __tron_current_command() {
    local entry
    entry=$(HISTTIMEFORMAT= builtin history 1)
    if [[ "$entry" =~ ^[[:space:]]*[0-9]+[[:space:]]+(.*)$ ]]; then
      entry="${BASH_REMATCH[1]}"
    fi
    printf '%s' "$entry"
  }

  __tron_in_command=0
  __tron_in_prompt=0

  __tron_preexec() {
    # The DEBUG trap also fires for our own hooks and for anything the user's
    # PROMPT_COMMAND runs; both must not open a command block.
    if [ "$__tron_in_prompt" = "1" ] || [ "$__tron_in_command" = "1" ]; then
      return
    fi
    case "$BASH_COMMAND" in
      __tron_*) return ;;
    esac
    __tron_in_command=1
    __tron_executed=1
    __tron_osc "633;E;$(__tron_escape "$(__tron_current_command)")"
    __tron_osc "133;C"
  }

  __tron_prompt_command() {
    local __tron_status=$?
    __tron_in_prompt=1
    if [ -n "${__tron_executed-}" ]; then
      __tron_osc "133;D;$__tron_status"
      unset __tron_executed
    fi
    __tron_report_cwd
    if [ -n "${__tron_original_prompt_command-}" ]; then
      eval "$__tron_original_prompt_command"
    fi
    __tron_in_prompt=0
    __tron_in_command=0
    return $__tron_status
  }

  if [ -z "${__TRON_PROMPT_WRAPPED-}" ]; then
    __TRON_PROMPT_WRAPPED=1
    # Works for both the scalar and the bash 5.1 array form of PROMPT_COMMAND.
    __tron_original_prompt_command="$(IFS=';'; printf '%s' "${PROMPT_COMMAND[*]-}")"
    PROMPT_COMMAND=__tron_prompt_command
    __tron_prompt_sync_end=
    if [ -n "${__TRON_SYNCED_BOOTSTRAP-}" ]; then
      __tron_prompt_sync_end='\[\e[?2026l\]'
      unset __TRON_SYNCED_BOOTSTRAP
    fi
    PS1="\[$(__tron_osc '133;A')\]${PS1}\[$(__tron_osc '133;B')\]${__tron_prompt_sync_end}"
    unset __tron_prompt_sync_end
    trap '__tron_preexec' DEBUG
  fi

  __tron_report_cwd

fi

if [ -n "${__TRON_SSH_HELPER-}" ] &&
  [ -n "${__TRON_SSH_INTEGRATION-}" ] &&
  [ -n "${__TRON_REAL_SSH-}" ] &&
  [ -n "${__TRON_SSH_BOOTSTRAP-}" ] &&
  [ -r "$__TRON_SSH_INTEGRATION" ] &&
  [ -r "$__TRON_SSH_HELPER" ] &&
  [ -r "$__TRON_SSH_BOOTSTRAP" ]; then
  unalias ssh 2>/dev/null || true
  ssh() {
    command /bin/sh "$__TRON_SSH_HELPER" "$@"
  }
fi
