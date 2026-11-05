// Reviewed adaptation for the pinned Microsoft language client (MIT licensed).
// Its two makeRe() callers use only truthiness, never the resulting RegExp.
// Minimatch 11 constructs a parsed set and matches directly without makeRe().
// Preserve rejection of empty/comment patterns using that set; do not restore
// regex compilation or modify the shared Minimatch prototype.
const fs = require('node:fs');
const path = require('node:path');
const { createHash } = require('node:crypto');

const clientRoot = path.resolve(path.dirname(require.resolve('vscode-languageclient/node')), '../..');
const manifest = JSON.parse(fs.readFileSync(path.join(clientRoot, 'package.json'), 'utf8'));
if (manifest.version !== '10.1.0') {
	throw new Error('Review the glob adaptation before changing vscode-languageclient version');
}
const originals = [
	['lib/common/fileOperations.js', '27021d92f7e2bb0094f0044d9516bfb3d76427e350b9a0945dac00e294f88f01'],
	['lib/common/utils/globPattern.js', 'ec8f8ed49366dc71c63a6908f0de06568086058cef3e59d6665df4c721826129'],
];
const before = 'if (!matcher.makeRe()) {';
const after = 'if (matcher.set.length === 0) { // Kujo: minimatch 11 parsed-pattern validation';
const digest = source => createHash('sha256').update(source).digest('hex');
// Validate every input before writing anything; reject unknown upstream edits.
const patches = originals.map(([relative, expected]) => {
	const file = path.join(clientRoot, relative);
	const source = fs.readFileSync(file, 'utf8');
	const original = source.includes(after) ? source.replace(after, before) : source;
	if (digest(original) !== expected || original.split(before).length !== 2) {
		throw new Error('Unrecognized language-client source: ' + relative);
	}
	return { file, source, patched: original.replace(before, after) };
});
for (const { file, source, patched } of patches) {
	if (source !== patched) fs.writeFileSync(file, patched);
}
console.log('Pinned language-client glob adaptation verified.');
