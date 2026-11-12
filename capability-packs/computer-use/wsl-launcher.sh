#!/bin/sh
# The Windows SDK runtime is part of this capability pack, never a global tool.
set -eu
read -r release < /proc/sys/kernel/osrelease
case "$release" in
    *[Mm]icrosoft*) ;;
    *) echo "Computer Use: this package requires WSL2." >&2; exit 1 ;;
esac
bin_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
runtime="$bin_dir/tron-computer-use-windows.exe"
if [ ! -f "$runtime" ]; then
    echo "Computer Use: the package is missing its Windows runtime." >&2
    exit 1
fi
chmod u+x -- "$runtime"
# The desktop backend removes Windows PATH entries from ordinary command tools.
# Its trusted launcher supplies this OS executable's path before filtering PATH.
powershell=${TRON_COMPUTER_USE_POWERSHELL:-powershell.exe}
system_root=$("$powershell" -NoLogo -NoProfile -NonInteractive -Command '[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); [Console]::Write($env:SystemRoot)')
windows_dir=$(wslpath -u "$system_root")
cd -- "$windows_dir"
exec "$runtime" "$@"
