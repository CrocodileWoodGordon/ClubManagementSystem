import { ApiClient } from "@/lib/api/client";
import type { Enrollment } from "@/lib/types";

const client = new ApiClient();

export async function fetchPendingEnrollments(): Promise<Enrollment[]> {
    const response = await client.get<{ data: Enrollment[] }>("/api/enrollments/pending");
    return response.data;
}
