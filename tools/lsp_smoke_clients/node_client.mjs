#!/usr/bin/env node
import { spawn } from 'node:child_process';

function writeMessage(stdin, payload) {
  const body = Buffer.from(JSON.stringify(payload), 'utf8');
  const header = Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, 'utf8');
  stdin.write(header);
  stdin.write(body);
}

function createReader(stdout) {
  let buffer = Buffer.alloc(0);
  const waiters = [];

  function drain() {
    while (waiters.length > 0) {
      const headerEnd = buffer.indexOf('\r\n\r\n');
      if (headerEnd === -1) {
        return;
      }

      const header = buffer.slice(0, headerEnd).toString('utf8');
      const lengthLine = header
        .split('\r\n')
        .find((line) => line.toLowerCase().startsWith('content-length:'));
      if (!lengthLine) {
        waiters.shift().reject(new Error('Missing Content-Length header'));
        return;
      }

      const contentLength = Number(lengthLine.split(':')[1].trim());
      const totalLength = headerEnd + 4 + contentLength;
      if (buffer.length < totalLength) {
        return;
      }

      const body = buffer.slice(headerEnd + 4, totalLength).toString('utf8');
      buffer = buffer.slice(totalLength);

      const waiter = waiters.shift();
      waiter.resolve(JSON.parse(body));
    }
  }

  stdout.on('data', (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    drain();
  });

  return {
    read() {
      return new Promise((resolve, reject) => {
        waiters.push({ resolve, reject });
        drain();
      });
    },
  };
}

async function main() {
  const binary = process.argv[2];
  if (!binary) {
    console.error('usage: node_client.mjs <kujo_binary>');
    process.exit(2);
  }

  const proc = spawn(binary, ['lsp'], { stdio: ['pipe', 'pipe', 'pipe'] });
  const reader = createReader(proc.stdout);

  writeMessage(proc.stdin, {
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {},
  });
  const init = await reader.read();
  if (!init.result) {
    throw new Error('initialize did not return result');
  }

  writeMessage(proc.stdin, {
    jsonrpc: '2.0',
    method: 'initialized',
    params: {},
  });

  writeMessage(proc.stdin, {
    jsonrpc: '2.0',
    id: 2,
    method: 'shutdown',
    params: null,
  });
  const shutdown = await reader.read();
  if (!Object.prototype.hasOwnProperty.call(shutdown, 'result')) {
    throw new Error('shutdown did not return result');
  }

  writeMessage(proc.stdin, {
    jsonrpc: '2.0',
    method: 'exit',
  });

  await new Promise((resolve, reject) => {
    proc.on('exit', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`kujo lsp exited with code ${code}`));
      }
    });
    proc.on('error', reject);
  });
}

main().catch((error) => {
  console.error(error.message);
  process.exit(1);
});
