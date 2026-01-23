import * as vscode from 'vscode';
import { IpcClient, DiagnosticsData } from './ipcClient';

export class DiagnosticsWatcher {
    private intervalId: NodeJS.Timeout | null = null;
    private lastTypingTime = 0;
    private lastDiagnostics: DiagnosticsData = { errors: 0, warnings: 0 };
    private disposables: vscode.Disposable[] = [];

    constructor(
        private ipcClient: IpcClient,
        private pollingInterval: number
    ) {}

    start(): void {
        this.intervalId = setInterval(() => this.collectDiagnostics(), this.pollingInterval);

        this.disposables.push(
            vscode.workspace.onDidChangeTextDocument(() => {
                const now = Date.now();
                if (now - this.lastTypingTime > 5000) {
                    this.ipcClient.sendActivity('typing');
                }
                this.lastTypingTime = now;
            })
        );

        this.disposables.push(
            vscode.workspace.onDidSaveTextDocument(() => {
                this.ipcClient.sendActivity('file_saved');
            })
        );

        this.disposables.push(
            vscode.tasks.onDidStartTask((e) => {
                const taskName = e.execution.task.name.toLowerCase();
                if (taskName.includes('build') || taskName.includes('compile')) {
                    this.ipcClient.sendActivity('build_started');
                }
            })
        );

        this.disposables.push(
            vscode.tasks.onDidEndTaskProcess((e) => {
                const taskName = e.execution.task.name.toLowerCase();
                if (taskName.includes('build') || taskName.includes('compile') || taskName.includes('cargo')) {
                    if (e.exitCode === 0) {
                        this.ipcClient.sendActivity('build_success');
                    } else {
                        this.ipcClient.sendActivity('build_failed');
                    }
                }
            })
        );

        this.collectDiagnostics();
    }

    stop(): void {
        if (this.intervalId) {
            clearInterval(this.intervalId);
            this.intervalId = null;
        }
        this.disposables.forEach(d => d.dispose());
        this.disposables = [];
    }

    private collectDiagnostics(): void {
        const diagnostics = vscode.languages.getDiagnostics();

        let errors = 0;
        let warnings = 0;

        for (const [, fileDiags] of diagnostics) {
            for (const diag of fileDiags) {
                switch (diag.severity) {
                    case vscode.DiagnosticSeverity.Error:
                        errors++;
                        break;
                    case vscode.DiagnosticSeverity.Warning:
                        warnings++;
                        break;
                }
            }
        }

        if (errors !== this.lastDiagnostics.errors || warnings !== this.lastDiagnostics.warnings) {
            this.lastDiagnostics = { errors, warnings };
            this.ipcClient.sendDiagnostics(this.lastDiagnostics);
        }
    }
}
