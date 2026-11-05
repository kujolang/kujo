const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { createRequire } = require('node:module');

// Load real client modules, substituting only the editor host API. An optional
// baseline root runs the same compatibility cases against the original client.
const root = process.env.KUJO_GLOB_CLIENT_ROOT || path.resolve(
	path.dirname(require.resolve('vscode-languageclient/node')), '../..');
const clientRequire = createRequire(path.join(root, 'package.json'));
const { Minimatch } = clientRequire('minimatch');
const uri = (fsPath, scheme = 'file') => ({ fsPath, scheme, path: fsPath,
	with: changes => uri(changes.path, scheme), toString: () => scheme + '://' + fsPath });
const host = {
	Uri: { parse(value) {
		const parsed = new URL(value);
		return uri(decodeURIComponent(parsed.pathname), parsed.protocol.slice(0, -1));
	} },
	FileType: { File: 1, Directory: 2 },
	workspace: {
		onDidCreateFiles: () => ({ dispose() {} }),
		fs: { stat: async item => ({ type: item.fsPath.endsWith('/dir') ? 2 : 1 }) },
	},
};
function load(relative) {
	const file = path.join(root, relative);
	const module = { exports: {} };
	const localRequire = createRequire(file);
	const execute = vm.runInThisContext('(function(require,module,exports){' + fs.readFileSync(file, 'utf8') + '\n})', { filename: file });
	execute(name => name === 'vscode' ? host : localRequire(name), module, module.exports);
	return module.exports;
}
const { matchGlobPattern } = load('lib/common/utils/globPattern.js');
const { DidCreateFilesFeature } = load('lib/common/fileOperations.js');
function feature() {
	return new DidCreateFilesFeature({ code2ProtocolConverter: { asDidCreateFilesParams: x => x }, info() {} });
}

test('real glob caller preserves string, relative URI and noext behavior', () => {
	for (const [pattern, name, expected] of [
		['**/*.kujo', '/src/main.kujo', true],
		['**/*.kujo', '/src/main.rs', false],
		['**/*.kujo', '/src/.hidden.kujo', false],
		['C:\\src\\*.kujo', 'C:/src/main.kujo', true],
		[{ baseUri: 'file:///src', pattern: '*.kujo' }, '/src/main.kujo', true],
		[{ baseUri: { uri: 'file:///src' }, pattern: '*.kujo' }, '/else/main.kujo', false],
		[{ baseUri: 'invalid URI', pattern: '*' }, '/src/main.kujo', false],
		['/src/+(main).kujo', '/src/main.kujo', false],
		['/src/+(main).kujo', '/src/+(main).kujo', true],
		['', '', false], ['#comment', '#comment', false],
		['/src/[abc', '/src/[abc', true],
		['/src/[z-a]', '/src/[z-a]', false],
		['/src/*.{kujo,rs}', '/src/main.rs', true],
		['!/src/*.rs', '/src/main.kujo', true],
	]) assert.equal(matchGlobPattern(pattern, uri(name)), expected, JSON.stringify([pattern, name]));
	assert.throws(() => matchGlobPattern('x'.repeat(65537), uri('/src/a')));
});

test('real file-operation registration/filter preserves path, scheme, kind and errors', async () => {
	const f = feature();
	f.register({ id: 'files', registerOptions: { filters: [
		{ scheme: 'file', pattern: { glob: '**/*.KUJO', options: { ignoreCase: true }, matches: 'file' } },
	] } });
	const files = [uri('/src/.hidden.kujo'), uri('C:\\src\\main.kujo'), uri('/src/main.rs'), uri('/src/main.kujo', 'untitled')];
	assert.deepEqual((await f.filter({ files }, x => x)).files, files.slice(0, 2));
	f.unregister('files');
	assert.equal(f.filterSize(), 0);
	f.register({ id: 'folders', registerOptions: { filters: [{ pattern: { glob: '**/dir/', matches: 'folder' } }] } });
	assert.equal((await f.filter({ files: [uri('/src/dir')] }, x => x)).files.length, 1);
	f.clear();
	for (const glob of ['', '#comment', 42, 'x'.repeat(65537)]) {
		assert.throws(() => f.register({ id: 'bad', registerOptions: { filters: [{ pattern: { glob } }] } }));
		assert.equal(f.filterSize(), 0);
	}
	for (const glob of ['**/[abc', '**/[z-a]']) {
		f.register({ id: 'literal', registerOptions: { filters: [{ pattern: { glob } }] } });
		assert.equal(f.filterSize(), 1);
		f.clear();
	}
});

test('patched minimatch bounds globparts amplification and brace expansion', { skip: Boolean(process.env.KUJO_GLOB_CLIENT_ROOT) }, () => {
	for (const pattern of ['**/../a/b/'.repeat(4) + 'x', 'a/**/../b/c/**/../d/e/x']) {
		const matcher = new Minimatch(pattern, { optimizationLevel: 2, braceExpandMax: 2 });
		assert.ok(matcher.globParts.length <= 2, 'globparts must respect the expansion limit');
	}
	assert.equal(new Minimatch('{a,b,c,d,e}', { braceExpandMax: 2 }).globParts.length, 2);
});

test('1242 client-option matching observations preserve the 10.2.5 baseline', () => {
	const patterns = ['', ' #literal', '#comment', '*', '**', '**/*.kujo', '/src/*', '/src/**',
		'**/.*', '**/*.{kujo,rs}', '!**/*.rs', '**/+(main|lib).kujo', '**/?(main).kujo',
		'**/!(main).kujo', '**/[a-z]*', '**/[!a]*', '**/[abc', '**/[z-a]', '**/a\\*b',
		'a/**/../b/c', 'C:/**/*.kujo', '//server/share/**/*.kujo', '**/café.kujo'];
	const paths = ['', 'a', 'b', 'a/b/c', '/src/main.kujo', '/src/lib.kujo', '/src/main.rs',
		'/src/.hidden.kujo', '/src/MAIN.KUJO', '/src/x/main.kujo', '/src/+(main).kujo',
		'/src/[abc', '/src/[z-a]', '/src/a*b', '/src/café.kujo', 'C:/src/main.kujo',
		'//server/share/main.kujo', '/src/dir/'];
	const results = [];
	for (const options of [{ dot: true }, { dot: true, nocase: true }, { noext: true }]) {
		for (const pattern of patterns) {
			const matcher = new Minimatch(pattern, options);
			const valid = typeof matcher.makeRe === 'function' ? Boolean(matcher.makeRe()) : matcher.set.length > 0;
			for (const name of paths) results.push(valid && matcher.match(name));
		}
	}
	assert.equal(results.length, 1242);
	// Captured from unmodified languageclient10.1.0/minimatch10.2.5; do not
	// regenerate from the candidate when upgrading. A mismatch needs review.
	assert.equal(require('node:crypto').createHash('sha256').update(JSON.stringify(results)).digest('hex'),
		'248b42760a483acb6918e96058b338e67cf953515c925326ae9b0446356d5703');
});

test('patch is idempotent and rejects modified inputs before writing', { skip: Boolean(process.env.KUJO_GLOB_CLIENT_ROOT) }, () => {
	const { spawnSync } = require('node:child_process');
	const directory = fs.mkdtempSync(path.join(require('node:os').tmpdir(), 'kujo-client-patch-'));
	try {
		const client = path.join(directory, 'node_modules/vscode-languageclient');
		fs.mkdirSync(path.join(client, 'lib/common/utils'), { recursive: true });
		fs.mkdirSync(path.join(client, 'lib/node'), { recursive: true });
		fs.writeFileSync(path.join(client, 'lib/node/main.js'), '');
		const manifest = { version: '10.1.0', exports: { './node': './lib/node/main.js' } };
		fs.writeFileSync(path.join(client, 'package.json'), JSON.stringify(manifest));
		const paths = ['lib/common/fileOperations.js', 'lib/common/utils/globPattern.js'];
		const before = 'if (!matcher.makeRe()) {';
		const after = 'if (matcher.set.length === 0) { // Kujo: minimatch 11 parsed-pattern validation';
		const originals = paths.map(p => fs.readFileSync(path.join(root, p), 'utf8').replace(after, before));
		paths.forEach((p, i) => fs.writeFileSync(path.join(client, p), originals[i]));
		const script = path.join(directory, 'patch.js');
		fs.copyFileSync(path.join(__dirname, 'patch-languageclient.js'), script);
		const run = () => spawnSync(process.execPath, [script], { encoding: 'utf8' });
		assert.equal(run().status, 0);
		assert.equal(run().status, 0);
		paths.forEach((p, i) => assert.equal(fs.readFileSync(path.join(client, p), 'utf8'), originals[i].replace(before, after)));
		fs.writeFileSync(path.join(client, paths[0]), originals[0]);
		fs.appendFileSync(path.join(client, paths[1]), '\n// unexpected edit');
		assert.match(run().stderr, /Unrecognized language-client source/);
		assert.equal(fs.readFileSync(path.join(client, paths[0]), 'utf8'), originals[0]);
		manifest.version = '11.0.0';
		fs.writeFileSync(path.join(client, 'package.json'), JSON.stringify(manifest));
		assert.match(run().stderr, /Review the glob adaptation/);
	} finally {
		fs.rmSync(directory, { recursive: true, force: true });
	}
});
