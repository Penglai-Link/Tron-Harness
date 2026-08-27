__tron_remote_dir=${TMPDIR:-/tmp}/tron-terminal-$$
__tron_remote_shell=${SHELL:-/bin/bash}
umask 077

if ! command mkdir "$__tron_remote_dir" 2>/dev/null; then
  exit 1
fi

if ! printf %s '__TRON_INTEGRATION_BASE64__' | {
  base64 -d 2>/dev/null || base64 -D
} >"$__tron_remote_dir/integration.sh"; then
  command rm -rf "$__tron_remote_dir"
  exit 1
fi

export __TRON_REMOTE_DIR="$__tron_remote_dir"
export __TRON_REMOTE_INTEGRATION="$__tron_remote_dir/integration.sh"

case "${__tron_remote_shell##*/}" in
  bash)
    cat >"$__tron_remote_dir/bashrc" <<'__TRON_BASH_RC__'
if [ -f /etc/bash.bashrc ]; then . /etc/bash.bashrc; fi
if [ -f "$HOME/.bashrc" ]; then . "$HOME/.bashrc"; fi
. "$__TRON_REMOTE_INTEGRATION"
command rm -rf "$__TRON_REMOTE_DIR"
unset __TRON_REMOTE_DIR __TRON_REMOTE_INTEGRATION
__TRON_BASH_RC__
    exec "$__tron_remote_shell" --init-file "$__tron_remote_dir/bashrc" -i
    ;;
  zsh)
    if [ -n "${ZDOTDIR-}" ]; then
      export __TRON_ORIGINAL_ZDOTDIR="$ZDOTDIR"
    else
      unset __TRON_ORIGINAL_ZDOTDIR
    fi
    cat >"$__tron_remote_dir/.zshenv" <<'__TRON_ZSHENV__'
if [ -f "${__TRON_ORIGINAL_ZDOTDIR:-$HOME}/.zshenv" ]; then
  . "${__TRON_ORIGINAL_ZDOTDIR:-$HOME}/.zshenv"
fi
__TRON_ZSHENV__
    cat >"$__tron_remote_dir/.zprofile" <<'__TRON_ZPROFILE__'
if [ -f "${__TRON_ORIGINAL_ZDOTDIR:-$HOME}/.zprofile" ]; then
  . "${__TRON_ORIGINAL_ZDOTDIR:-$HOME}/.zprofile"
fi
__TRON_ZPROFILE__
    cat >"$__tron_remote_dir/.zshrc" <<'__TRON_ZSHRC__'
if [ -n "${__TRON_ORIGINAL_ZDOTDIR-}" ]; then
  ZDOTDIR="$__TRON_ORIGINAL_ZDOTDIR"
else
  unset ZDOTDIR
fi
if [ -f "${ZDOTDIR:-$HOME}/.zshrc" ]; then . "${ZDOTDIR:-$HOME}/.zshrc"; fi
. "$__TRON_REMOTE_INTEGRATION"
command rm -rf "$__TRON_REMOTE_DIR"
unset __TRON_ORIGINAL_ZDOTDIR __TRON_REMOTE_DIR __TRON_REMOTE_INTEGRATION
__TRON_ZSHRC__
    ZDOTDIR="$__tron_remote_dir"
    export ZDOTDIR
    exec "$__tron_remote_shell" -l
    ;;
  *)
    command rm -rf "$__tron_remote_dir"
    exec "$__tron_remote_shell" -l
    ;;
esac
