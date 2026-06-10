const path = require('path');
const vscode = require('vscode');
const {
	LanguageClient,
	TransportKind,
	Trace,
} = require('vscode-languageclient/node');

let client;

function parseTraceLevel(value) {
	if ('messages' === value) {
		return Trace.Messages;
	}
	if ('verbose' === value) {
		return Trace.Verbose;
	}
	return Trace.Off;
}

function createServerOptions(command_config) {
	if (!Array.isArray(command_config) || 0 === command_config.length) {
		return null;
	}

	const command = String(command_config[0]);
	const args = command_config.slice(1).map(String);

	return {
		run: {
			command,
			args,
			transport: TransportKind.stdio,
		},
		debug: {
			command,
			args,
			transport: TransportKind.stdio,
		},
	};
}

function activate(context) {
	const config = vscode.workspace.getConfiguration('kujo');
	if (!config.get('lsp.enabled', true)) {
		return;
	}

	const command_config = config.get('lsp.command', ['kujo', 'lsp']);
	const server_options = createServerOptions(command_config);
	if (null === server_options) {
		vscode.window.showErrorMessage('Kujo LSP command is not configured. Set kujo.lsp.command to an array like [\'kujo\', \'lsp\'].');
		return;
	}

	const client_options = {
		documentSelector: [
			{ scheme: 'file', language: 'kujo' },
		],
		synchronize: {
			fileEvents: vscode.workspace.createFileSystemWatcher('**/*.kujo'),
		},
	};

	client = new LanguageClient(
		'kujoLanguageServer',
		'Kujo Language Server',
		server_options,
		client_options
	);

	const trace_setting = config.get('lsp.trace.server', 'off');
	client.setTrace(parseTraceLevel(trace_setting));

	context.subscriptions.push(client.start());
}

async function deactivate() {
	if (!client) {
		return;
	}
	await client.stop();
}

module.exports = {
	activate,
	deactivate,
};
