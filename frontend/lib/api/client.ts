export interface ApiClientOptions {
    baseUrl?: string;
}

export class ApiClient {
    private readonly baseUrl: string;

    constructor(options: ApiClientOptions = {}) {
        this.baseUrl = options.baseUrl ?? process.env.NEXT_PUBLIC_API_URL ?? "/api";
    }

    async get<T>(path: string): Promise<T> {
        const res = await fetch(`${this.baseUrl}${path}`, { cache: "no-store" });
        if (!res.ok) {
            throw new Error(`GET ${path} failed`);
        }
        return res.json();
    }

    async post<T>(path: string, body: unknown): Promise<T> {
        const res = await fetch(`${this.baseUrl}${path}`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
        });
        if (!res.ok) {
            throw new Error(`POST ${path} failed`);
        }
        return res.json();
    }

    async put<T>(path: string, body: unknown): Promise<T> {
        const res = await fetch(`${this.baseUrl}${path}`, {
            method: "PUT",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
        });
        if (!res.ok) {
            throw new Error(`PUT ${path} failed`);
        }
        return res.json();
    }

    async delete(path: string): Promise<void> {
        const res = await fetch(`${this.baseUrl}${path}`, { method: "DELETE" });
        if (!res.ok) {
            throw new Error(`DELETE ${path} failed`);
        }
    }
}
