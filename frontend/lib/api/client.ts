export interface ApiClientOptions {
    baseUrl?: string;
}

export class ApiClient {
    private readonly baseUrl: string;

    constructor(options: ApiClientOptions = {}) {
        this.baseUrl = options.baseUrl ?? process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";
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
}
