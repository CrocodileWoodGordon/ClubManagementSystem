import type { HomeroomBillingReport } from "@/lib/types";
import { formatWaiverReason } from "@/lib/utils";

export function escapeCsv(value: string): string {
    if (value.includes(",") || value.includes('"') || value.includes("\n")) {
        return `"${value.replace(/"/g, '""')}"`;
    }
    return value;
}

export function buildCsv(headers: string[], rows: string[][]): string {
    return [headers, ...rows]
        .map((line) => line.map((cell) => escapeCsv(cell ?? "")).join(","))
        .join("\n");
}

export function downloadCsv(content: string, fileName: string) {
    const blob = new Blob([content], { type: "text/csv;charset=utf-8;" });
    downloadBlob(blob, fileName);
}

export function exportCsv(headers: string[], rows: string[][], fileName: string) {
    const csv = buildCsv(headers, rows);
    downloadCsv(csv, fileName);
}

export function exportHomeroomBillingExcel(
    report: HomeroomBillingReport,
    fileName: string,
) {
    const rows = buildHomeroomExcelRows(report);
    const html = buildExcelHtml(report, rows);
    const blob = new Blob([html], {
        type: "application/vnd.ms-excel;charset=utf-8;",
    });
    downloadBlob(blob, fileName);
}

function buildHomeroomExcelRows(report: HomeroomBillingReport): string[][] {
    const header = [
        "学生姓名",
        "学号",
        "社团",
        "班级编号",
        "班级 ID",
        "出勤次数",
        "计费课次",
        "课时费",
        "材料费",
        "减免",
        "应缴金额",
        "减免原因",
        "备注",
    ];
    const rows: string[][] = [header];

    report.students.forEach((student) => {
        if (student.rows.length === 0) {
            rows.push([
                student.studentName,
                student.studentCode ?? "",
                "无社团记录",
                "",
                "",
                "0",
                "0",
                "0.00",
                "0.00",
                "0.00",
                "0.00",
                "",
                "",
            ]);
            return;
        }

        let lessonTotal = 0;
        let materialTotal = 0;
        let discountTotal = 0;

        student.rows.forEach((item) => {
            lessonTotal += item.lessonFee;
            materialTotal += item.materialFee;
            discountTotal += item.discountAmount;
            rows.push([
                student.studentName,
                student.studentCode ?? "",
                item.clubName,
                item.classCode ?? "",
                item.classId,
                String(item.attendanceCount),
                String(item.chargedSessions),
                item.lessonFee.toFixed(2),
                item.materialFee.toFixed(2),
                item.discountAmount.toFixed(2),
                (item.lessonFee + item.materialFee).toFixed(2),
                formatWaiverReason(item.waiveReason),
                item.remarks ?? "",
            ]);
        });

        rows.push([
            student.studentName,
            student.studentCode ?? "",
            "合计",
            "",
            "",
            "",
            "",
            lessonTotal.toFixed(2),
            materialTotal.toFixed(2),
            discountTotal.toFixed(2),
            (lessonTotal + materialTotal).toFixed(2),
            "",
            "",
        ]);
    });

    return rows;
}

function buildExcelHtml(report: HomeroomBillingReport, rows: string[][]) {
    const title = `${report.homeroom.displayName}-${report.homeroom.campusName}`;
    const tableBody = rows
        .map(
            (row) =>
                `<tr>${row
                    .map((cell) => `<td>${escapeHtml(cell)}</td>`)
                    .join("")}</tr>`,
        )
        .join("");
    return `<!DOCTYPE html><html><head><meta charset="UTF-8"><title>${escapeHtml(title)}</title></head><body><table>${tableBody}</table></body></html>`;
}

function downloadBlob(blob: Blob, fileName: string) {
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = sanitizeFileName(fileName);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
}

function sanitizeFileName(value: string) {
    return value.replace(/[\/:*?"<>|]/g, "_");
}

function escapeHtml(value: string) {
    return value
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
}
