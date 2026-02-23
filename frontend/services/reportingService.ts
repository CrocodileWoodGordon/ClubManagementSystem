import { ApiClient } from "@/lib/api/client";
import type {
    FeeBreakdown,
    HomeroomBillingReport,
    StudentBillingItem,
    TuitionWaiverReason,
} from "@/lib/types";

const client = new ApiClient();

export class ReportingServiceError extends Error {
    constructor(message: string) {
        super(message);
        this.name = "ReportingServiceError";
    }
}

export async function fetchClassSettlement(classId: string): Promise<FeeBreakdown[]> {
    if (!classId) {
        throw new ReportingServiceError("请先选择班级");
    }
    return safeRequest("加载班级结算失败", async () => {
        const response = await client.get<SettlementResponse>(
            `/api/reports/settlement?class_id=${encodeURIComponent(classId)}`,
        );
        return response.data.map(mapBreakdown);
    });
}

export async function fetchStudentBilling(studentId: string): Promise<FeeBreakdown[]> {
    if (!studentId) {
        throw new ReportingServiceError("请先选择学生");
    }
    return safeRequest("加载学生账单失败", async () => {
        const response = await client.get<SettlementResponse>(
            `/api/reports/billing?student_id=${encodeURIComponent(studentId)}`,
        );
        return response.data.map(mapBreakdown);
    });
}

export async function fetchHomeroomBilling(
    homeroomId: string,
): Promise<HomeroomBillingReport> {
    if (!homeroomId) {
        throw new ReportingServiceError("请先选择班级");
    }
    return safeRequest("加载整班账单失败", async () => {
        const response = await client.get<HomeroomBillingApiResponse>(
            `/api/reports/billing/homeroom?homeroom_id=${encodeURIComponent(homeroomId)}`,
        );
        return {
            homeroom: {
                id: response.homeroom.id,
                displayName: response.homeroom.display_name,
                campusName: response.homeroom.campus_name,
            },
            students: response.students.map((student) => ({
                studentId: student.student_id,
                studentName: student.student_name,
                studentCode: toOptional(student.student_code),
                rows: student.rows.map(mapStudentBreakdown),
            })),
        };
    });
}

async function safeRequest<T>(
    context: string,
    executor: () => Promise<T>,
): Promise<T> {
    try {
        return await executor();
    } catch (error) {
        if (error instanceof ReportingServiceError) {
            throw error;
        }
        const message = error instanceof Error ? error.message : String(error);
        throw new ReportingServiceError(`${context}: ${message}`);
    }
}

function mapBreakdown(payload: FeeBreakdownApi): FeeBreakdown {
    return {
        enrollmentId: payload.enrollment_id,
        studentId: payload.student_id,
        classId: payload.class_id,
        materialFee: Number(payload.material_fee),
        lessonFee: Number(payload.lesson_fee),
        discountAmount: Number(payload.discount_amount),
        attendanceCount: payload.attendance_count,
        chargedSessions: payload.charged_sessions,
        waiveReason: mapWaiverReason(payload.waive_reason),
        remarks: toOptional(payload.remarks),
    };
}

function mapStudentBreakdown(payload: StudentBillingItemApi): StudentBillingItem {
    const breakdown = mapBreakdown(payload);
    return {
        ...breakdown,
        clubId: payload.club_id,
        clubName: payload.club_name,
        classCode: toOptional(payload.class_code),
    };
}

function toOptional<T>(value: T | null | undefined): T | undefined {
    return value === null || value === undefined ? undefined : value;
}

const WAIVER_REASON_MAP: Record<string, TuitionWaiverReason> = {
    DROP_WITHIN_GRACE: "DROP_WITHIN_GRACE",
    MANUAL_OVERRIDE: "MANUAL_OVERRIDE",
    TEACHER_BENEFIT: "TEACHER_BENEFIT",
};

function mapWaiverReason(value: string | null): TuitionWaiverReason | undefined {
    if (!value) {
        return undefined;
    }
    return WAIVER_REASON_MAP[value] ?? undefined;
}

interface SettlementResponse {
    data: FeeBreakdownApi[];
}

interface FeeBreakdownApi {
    enrollment_id: string;
    student_id: string;
    class_id: string;
    material_fee: number;
    lesson_fee: number;
    discount_amount: number;
    attendance_count: number;
    charged_sessions: number;
    waive_reason: string | null;
    remarks: string | null;
}

interface HomeroomBillingApiResponse {
    homeroom: HomeroomInfoApi;
    students: StudentBillingBundleApi[];
}

interface HomeroomInfoApi {
    id: string;
    display_name: string;
    campus_name: string;
}

interface StudentBillingBundleApi {
    student_id: string;
    student_name: string;
    student_code: string | null;
    rows: StudentBillingItemApi[];
}

interface StudentBillingItemApi extends FeeBreakdownApi {
    club_id: string;
    club_name: string;
    class_code: string | null;
}
