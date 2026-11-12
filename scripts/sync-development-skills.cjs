const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const { parseArgs } = require('node:util');

const root = path.resolve(__dirname, '..');
const destination = path.join(root, 'tron-backend/crates/eliteforge-agent-capability-development');
const digest = data => crypto.createHash('sha256').update(data).digest('hex');
function readFiles(directory, prefix = '', files = new Map()) {
  if (fs.lstatSync(directory).isSymbolicLink()) throw new Error(`不允许符号链接：${directory}`);
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (!prefix && entry.name === 'snapshot.json') continue;
    const name = prefix + entry.name;
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) readFiles(absolute, `${name}/`, files);
    else if (entry.isFile()) files.set(name, fs.readFileSync(absolute));
    else throw new Error(`不支持的文件类型：${absolute}`);
  }
  return files;
}
function coreDependency() {
  const manifest = fs.readFileSync(path.join(root, 'tron-backend/Cargo.toml'), 'utf8');
  const dependency = manifest.match(/^eliteforge-agent-core = (\{[^\n]+\})$/m)?.[1];
  if (!dependency || !dependency.includes('git = ') || !/\b(?:rev|tag) = "[^"\n]+"/.test(dependency)) throw new Error('宿主缺少固定 Git 版本的 core 依赖');
  return dependency;
}
function alignTemplateDependencies(files, dependency) {
  const url = dependency.match(/git = "([^"]+)"/)[1];
  const reference = dependency.match(/\b(rev|tag) = "([^"\n]+)"/);
  const sourcePrefix = `source = "git+${url}?${reference[1]}=${reference[2]}#`;
  const lockedSource = fs.readFileSync(path.join(root, 'tron-backend/Cargo.lock'), 'utf8')
    .split('\n').find(line => line.startsWith(sourcePrefix));
  if (!lockedSource) throw new Error('请先更新宿主 Cargo.lock，再同步开发技能');
  for (const [name, data] of files) {
    let text = data.toString();
    if (name.endsWith('Cargo.toml') || name.endsWith('Cargo.toml.tpl')) {
      text = text.split('\n').map(line => line.includes(`git = "${url}"`)
        ? line.replace(/\b(?:rev|tag) = "[^"\n]+"/, reference[0]) : line).join('\n');
    } else if (name.endsWith('Cargo.lock')) {
      text = text.split('\n').map(line => line.startsWith(`source = "git+${url}?`)
        ? lockedSource : line).join('\n');
    } else if (name.endsWith('docs/pack-contract.md')) {
      text = text.replace(/Rust 依赖固定到 `[^`]+`/, 'Rust 依赖与宿主使用相同的 Framework Git 标签或提交');
    } else continue;
    files.set(name, Buffer.from(text));
  }
}
function verifySnapshot(files, snapshot) {
  if (snapshot.source !== 'eliteforge-agent-framework/crates/capability/eliteforge-agent-capability-development') throw new Error('快照来源不匹配');
  if (files.size !== Object.keys(snapshot.files).length || [...files].some(([name, data]) => snapshot.files[name] !== digest(data))) {
    throw new Error('开发技能快照有本地修改，请先将改动合入 Framework 源');
  }
}
try {
  const { values } = parseArgs({ options: { check: { type: 'boolean' }, source: { type: 'string' } } });
  if (values.check && values.source) throw new Error('--check 不接受 --source');
  let current = new Map();
  if (fs.existsSync(destination)) {
    current = readFiles(destination);
    const recordPath = path.join(destination, 'snapshot.json');
    if (!fs.lstatSync(recordPath).isFile()) throw new Error('快照清单必须是普通文件');
    verifySnapshot(current, JSON.parse(fs.readFileSync(recordPath, 'utf8')));
  } else if (values.check) throw new Error('缺少开发技能快照，请执行 make development-skills-sync');
  const dependency = coreDependency();
  if (values.check) {
    const matches = [...current.get('Cargo.toml').toString().matchAll(/^eliteforge-agent-core = (.+)$/gm)];
    if (matches.length !== 2 || matches.some(match => match[1] !== dependency)) throw new Error('快照 core 版本与宿主不同，请重新同步');
    console.log(`开发技能快照完整：${current.size} 个文件；core 版本与宿主一致`);
  } else {
    const framework = fs.realpathSync(path.resolve(root, values.source || '../eliteforge-agent-framework'));
    const source = path.join(framework, 'crates/capability/eliteforge-agent-capability-development');
    const files = readFiles(source);
    const workspace = fs.readFileSync(path.join(framework, 'Cargo.toml'), 'utf8').match(/\[workspace\.package\]([\s\S]*?)(?=\n\[|$)/)?.[1];
    if (!workspace) throw new Error('Framework 缺少 workspace.package');
    let manifest = files.get('Cargo.toml').toString();
    if (!/^name = "eliteforge-agent-capability-development"$/m.test(manifest)) throw new Error('Framework 开发包名称不匹配');
    for (const field of ['version', 'edition', 'license', 'repository']) {
      const value = workspace.match(new RegExp(`^${field} = ("[^"\\n]+")$`, 'm'))?.[1];
      if (!value || !manifest.includes(`${field}.workspace = true`)) throw new Error(`无法解析 workspace 字段 ${field}`);
      manifest = manifest.replace(`${field}.workspace = true`, `${field} = ${value}`);
    }
    let count = 0;
    manifest = manifest.replace(/^eliteforge-agent-core = \{ path = "\.\.\/\.\.\/framework\/eliteforge-agent-core" \}$/gm, () => {
      count += 1;
      return `eliteforge-agent-core = ${dependency}`;
    });
    if (count !== 2) throw new Error('Framework 开发包的 core 依赖结构已变化，请更新同步命令');
    files.set('Cargo.toml', Buffer.from(manifest));
    alignTemplateDependencies(files, dependency);
    const snapshot = {
      source: 'eliteforge-agent-framework/crates/capability/eliteforge-agent-capability-development',
      files: Object.fromEntries([...files].sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0).map(([name, data]) => [name, digest(data)])),
    };
    // Delete only files verified against the previous snapshot above.
    for (const name of current.keys()) if (!files.has(name)) fs.unlinkSync(path.join(destination, name));
    for (const [name, data] of files) {
      const target = path.join(destination, name);
      fs.mkdirSync(path.dirname(target), { recursive: true });
      fs.writeFileSync(target, data);
    }
    fs.writeFileSync(path.join(destination, 'snapshot.json'), `${JSON.stringify(snapshot, null, 2)}\n`);
    console.log(`已同步共用开发技能：${files.size} 个文件`);
  }
} catch (error) {
  console.error(`错误：${error.message}`);
  process.exitCode = 1;
}
