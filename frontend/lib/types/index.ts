export interface Student {
    id: string;
    name: string;
    originalClass: string;
    isTeacherChild: boolean;
}

export interface ClassInstance {
    id?: string;
    clubId: string;
    dayOfWeek: number;
    batchNumber: string;
    timeSlot: string;
    location: string;
}

export type EnrollmentStatus = "PENDING" | "ACTIVE" | "DROPPED" | "TRANSFERRED_OUT" | "TRANSFERRED_IN";

export interface PendingEnrollment {
    enrollmentId: string;
    studentId: string;
    studentName: string;
    studentCode?: string;
    homeroom: string;
    campusId: string;
    campusName: string;
    clubId: string;
    clubName: string;
    requestedWeekday: number;
    status: EnrollmentStatus;
}

export interface EnrollmentSummaryRow {
    campusId: string;
    campusName: string;
    clubId: string;
    clubName: string;
    requestedWeekday: number;
    total: number;
}
