import type { AttendanceStatus, EnrollmentStatus } from "@/lib/types";

const STATUS_LABELS: Record<EnrollmentStatus, string> = {
    PENDING: "待分班",
    ACTIVE: "已激活",
    DROPPED: "已退课",
    TRANSFERRED_OUT: "已转出",
    TRANSFERRED_IN: "转入待确认",
};

const ATTENDANCE_STATUS_LABELS: Record<AttendanceStatus, string> = {
    PRESENT: "出勤",
    ABSENT: "缺勤",
    EXCUSED: "病假",
    LEAVE: "事假",
};

export function formatWeekday(dayOfWeek: number) {
    const map = ["一", "二", "三", "四", "五", "六", "日"];
    return `周${map[dayOfWeek - 1] ?? "?"}`;
}

export function formatEnrollmentStatus(status: EnrollmentStatus) {
    return STATUS_LABELS[status] ?? status;
}

export function formatAttendanceStatus(status: AttendanceStatus) {
    return ATTENDANCE_STATUS_LABELS[status] ?? status;
}
