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
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = fileName;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
}

export function exportCsv(headers: string[], rows: string[][], fileName: string) {
    const csv = buildCsv(headers, rows);
    downloadCsv(csv, fileName);
}
