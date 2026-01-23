import * as vscode from 'vscode';
import { DiagnosticsWatcher } from './diagnosticsWatcher';
import { IpcClient } from './ipcClient';

let watcher: DiagnosticsWatcher | undefined;
let ipcClient: IpcClient | undefined;
let statusBarItem: vscode.StatusBarItem | undefined;

export function activate(context: vscode.ExtensionContext) {
    console.log('Panda Pet extension is now active');

    const config = vscode.workspace.getConfiguration('pandaPet');
    const enabled = config.get<boolean>('enabled', true);

    if (!enabled) {
        console.log('Panda Pet is disabled');
        return;
    }

    const socketPath = config.get<string>('socketPath', '/tmp/panda-pet.sock');
    const pollingInterval = config.get<number>('pollingInterval', 2000);

    ipcClient = new IpcClient(socketPath);
    ipcClient.connect();

    watcher = new DiagnosticsWatcher(ipcClient, pollingInterval);
    watcher.start();

    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBarItem.text = '$(squirrel) Panda';
    statusBarItem.tooltip = 'Panda Pet - Click to show status';
    statusBarItem.command = 'pandaPet.showStatus';
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);

    context.subscriptions.push(
        vscode.commands.registerCommand('pandaPet.startBreak', () => {
            ipcClient?.sendBreakTime(true);
            vscode.window.showInformationMessage('Panda is taking a snack break! 🐼🎋');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('pandaPet.endBreak', () => {
            ipcClient?.sendBreakTime(false);
            vscode.window.showInformationMessage('Panda is back to work! 🐼');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('pandaPet.showStatus', () => {
            const connected = ipcClient?.isConnected() ?? false;
            const status = connected ? 'Connected to daemon' : 'Daemon not running';
            vscode.window.showInformationMessage(`Panda Pet: ${status}`);
        })
    );

    vscode.workspace.onDidChangeConfiguration(e => {
        if (e.affectsConfiguration('pandaPet')) {
            vscode.window.showInformationMessage('Panda Pet: Reload window to apply changes');
        }
    });

    context.subscriptions.push({
        dispose: () => {
            watcher?.stop();
            ipcClient?.disconnect();
        }
    });
}

export function deactivate() {
    watcher?.stop();
    ipcClient?.disconnect();
}
