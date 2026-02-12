import { ApiClient } from "@/lib/api/client";
import type { FeeBreakdown } from "@/lib/types";

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
        waiveReason: toOptional(payload.waive_reason),
        remarks: toOptional(payload.remarks),
    };
}

function toOptional<T>(value: T | null | undefined): T | undefined {
    return value === null || value === undefined ? undefined : value;
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
