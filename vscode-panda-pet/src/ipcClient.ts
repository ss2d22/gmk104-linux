import * as net from 'net';

export interface DiagnosticsData {
    errors: number;
    warnings: number;
}

export type ActivityEvent = 'typing' | 'file_saved' | 'build_started' | 'build_success' | 'build_failed';

export class IpcClient {
    private socket: net.Socket | null = null;
    private socketPath: string;
    private reconnectTimer: NodeJS.Timeout | null = null;
    private connected = false;

    constructor(socketPath: string) {
        this.socketPath = socketPath;
    }

    connect(): void {
        if (this.socket) {
            return;
        }

        this.socket = new net.Socket();

        this.socket.on('connect', () => {
            console.log('Connected to panda pet daemon');
            this.connected = true;
        });

        this.socket.on('error', (err) => {
            if ((err as NodeJS.ErrnoException).code !== 'ENOENT') {
                console.log('Panda pet daemon connection error:', err.message);
            }
            this.connected = false;
        });

        this.socket.on('close', () => {
            this.connected = false;
            this.socket = null;
            this.scheduleReconnect();
        });

        this.socket.connect(this.socketPath);
    }

    private scheduleReconnect(): void {
        if (this.reconnectTimer) {
            return;
        }
        this.reconnectTimer = setTimeout(() => {
            this.reconnectTimer = null;
            this.connect();
        }, 5000);
    }

    disconnect(): void {
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }
        if (this.socket) {
            this.socket.destroy();
            this.socket = null;
        }
        this.connected = false;
    }

    isConnected(): boolean {
        return this.connected;
    }

    private send(message: object): void {
        if (!this.socket || !this.connected) {
            return;
        }
        try {
            const json = JSON.stringify(message) + '\n';
            this.socket.write(json);
        } catch (err) {
            console.error('Failed to send message:', err);
        }
    }

    sendDiagnostics(data: DiagnosticsData): void {
        this.send({
            type: 'diagnostics',
            errors: data.errors,
            warnings: data.warnings,
        });
    }

    sendActivity(event: ActivityEvent): void {
        this.send({
            type: 'activity',
            event: event,
        });
    }

    sendBreakTime(active: boolean): void {
        this.send({
            type: 'break',
            active: active,
        });
    }
}
