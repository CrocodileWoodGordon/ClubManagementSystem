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

export interface Enrollment {
    id: string;
    studentId: string;
    classId?: string;
    status: "PENDING" | "ACTIVE" | "DROPPED" | "TRANSFERRED";
}
