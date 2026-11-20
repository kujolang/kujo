const assert = require('node:assert/strict');
const fs = require('node:fs');
const vscode = require('vscode');

exports.run = async () => {
	const extension = vscode.extensions.getExtension('kujolang.kujo-language-tools');
	assert.ok(extension, 'Kujo extension is loaded in the VS Code host');
	const workspace = vscode.workspace.workspaceFolders[0].uri;
	const path = vscode.Uri.joinPath(workspace, 'kujo-host-smoke.kujo');
	await extension.activate();
	await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(path));
	const hover = await vscode.commands.executeCommand(
		'vscode.executeHoverProvider', path, new vscode.Position(3, 1)
	);
	assert.ok(hover?.length, 'Kujo must return a hover result over the stdio client');
	fs.writeFileSync(vscode.Uri.joinPath(workspace, 'kujo-host-smoke.passed').fsPath, 'passed\n');
};
