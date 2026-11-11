set -eu
export DEBIAN_FRONTEND=noninteractive
sed -i 's|http://deb.debian.org|https://deb.debian.org|g' /etc/apt/sources.list.d/debian.sources
for wsl_attempt in 1 2 3; do
    if apt-get -o Acquire::Retries=3 -o APT::Update::Error-Mode=any update && apt-get -o Acquire::Retries=3 install -y --no-install-recommends cmake libwayland-dev; then
        break
    fi
    if [ "$wsl_attempt" -eq 3 ]; then exit 1; fi
    echo "构建依赖下载失败，重试安装（已尝试 $wsl_attempt 次）。" >&2
done
export CARGO_HOME=/cargo-cache
export CARGO_TARGET_DIR=/build
export RUSTC_WRAPPER=
export RUSTC_WORKSPACE_WRAPPER=
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS CARGO_BUILD_TARGET
cargo build --locked --release --manifest-path tron-backend/Cargo.toml --bin tron-backend --target "$TRON_WSL_TARGET"
wsl_binary="/build/$TRON_WSL_TARGET/release/tron-backend"
test -x "$wsl_binary"
wsl_stage=$(mktemp -d /output/.build-XXXXXXXX)
trap 'rm -rf -- "$wsl_stage"' EXIT
cp "$wsl_binary" "$wsl_stage/tron-backend"
chmod 755 "$wsl_stage/tron-backend"
ldd "$wsl_stage/tron-backend" > "$wsl_stage/runtime-dependencies.txt"
if grep -q 'not found' "$wsl_stage/runtime-dependencies.txt"; then
    cat "$wsl_stage/runtime-dependencies.txt" >&2
    echo "错误：构建产物存在未解析的动态库。" >&2
    exit 1
fi
printf '\n构建镜像：%s\n版本：%s\n目标：%s\n运行基线：WSL2 Ubuntu 24.04+、Debian 13+\n' "$TRON_WSL_IMAGE" "$TRON_WSL_VERSION" "$TRON_WSL_TARGET" >> "$wsl_stage/runtime-dependencies.txt"
printf '系统运行库：libc6、libgcc-s1、libstdc++6、libssl3t64、libwayland-client0、ca-certificates。\n' >> "$wsl_stage/runtime-dependencies.txt"
printf '以上 ldd 只列出主程序依赖；内嵌 SQLite 扩展还要求 GLIBC_2.38、GLIBCXX_3.4.32。Debian 12 不满足运行要求。\n' >> "$wsl_stage/runtime-dependencies.txt"
cd "$wsl_stage"
sha256sum tron-backend > SHA256SUMS
chown -R "$TRON_WSL_UID:$TRON_WSL_GID" "$wsl_stage"
for wsl_file in tron-backend runtime-dependencies.txt SHA256SUMS; do
    mv -f "$wsl_file" "/output/$wsl_file"
done
