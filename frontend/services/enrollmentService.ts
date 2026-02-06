import { ApiClient } from "@/lib/api/client";
import type { EnrollmentSummaryRow, PendingEnrollment } from "@/lib/types";

const client = new ApiClient();

export interface EnrollmentListParams {
    termId?: string;
    campusId?: string;
    homeroom?: string;
    club?: string;
    weekday?: number;
    studentName?: string;
}

export async function fetchPendingEnrollments(
    params?: EnrollmentListParams,
): Promise<PendingEnrollment[]> {
    const search = buildQueryString({
        term_id: params?.termId,
        campus_id: params?.campusId,
        homeroom: params?.homeroom,
        club: params?.club,
        weekday: params?.weekday !== undefined ? String(params.weekday) : undefined,
        student_name: params?.studentName,
    });
    const response = await client.get<{ data: PendingEnrollment[] }>(
        `/api/enrollments/pending${search}`,
    );
    return response.data;
}

export interface EnrollmentSummaryParams {
    termId?: string;
    campusId?: string;
}

export async function fetchEnrollmentSummary(
    params?: EnrollmentSummaryParams,
): Promise<EnrollmentSummaryRow[]> {
    const search = buildQueryString({
        term_id: params?.termId,
        campus_id: params?.campusId,
    });
    const response = await client.get<{ data: EnrollmentSummaryRow[] }>(
        `/api/enrollments/summary${search}`,
    );
    return response.data;
}

function buildQueryString(values: Record<string, string | undefined>): string {
    const search = new URLSearchParams();
    Object.entries(values).forEach(([key, value]) => {
        if (value && value.length > 0) {
            search.set(key, value);
        }
    });
    const query = search.toString();
    return query ? `?${query}` : "";
}
