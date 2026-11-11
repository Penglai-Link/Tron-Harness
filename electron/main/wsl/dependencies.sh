set -eu

# A fresh desktop installation does not imply missing Linux dependencies.
# Avoid contacting APT (or upgrading installed packages) when WSL is ready.
set -- python3 util-linux git ca-certificates libgcc-s1 libstdc++6 libssl3t64 libwayland-client0
missing=""
for package do
    if [ "$(dpkg-query -W -f='${db:Status-Status}' "$package" 2>/dev/null)" != installed ]; then
        missing="$missing $package"
    fi
done
if [ -z "$missing" ]; then
    exit 0
fi

export DEBIAN_FRONTEND=noninteractive
# WSL commonly has no IPv6 route. Bound connection attempts rather than
# leaving the environment switch waiting for APT's default network timeout.
apt_install() {
    apt-get -o Acquire::ForceIPv4=true -o Acquire::Retries=2 \
        -o Acquire::http::Timeout=15 -o Acquire::https::Timeout=15 \
        -o DPkg::Lock::Timeout=120 "$@"
}
dependency_error() {
    printf '\nWSL 缺少运行依赖：%s。%s失败，请检查该发行版的软件源、网络或代理配置后重试。\n' "$missing" "$1" >&2
    exit 1
}
if ! apt_install -o APT::Update::Error-Mode=any update; then
    dependency_error 'APT 索引更新'
fi
# Only fixed package names collected above are split into arguments here.
if ! apt_install install -y --no-install-recommends $missing; then
    dependency_error 'APT 依赖安装'
fi
for package do
    if [ "$(dpkg-query -W -f='${db:Status-Status}' "$package" 2>/dev/null)" != installed ]; then
        dependency_error '依赖安装后检查'
    fi
done
