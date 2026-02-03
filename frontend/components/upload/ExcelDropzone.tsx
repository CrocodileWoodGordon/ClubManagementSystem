"use client";

interface ExcelDropzoneProps {
    onFileSelected: (file: File) => void;
}

export function ExcelDropzone({ onFileSelected }: ExcelDropzoneProps) {
    return (
        <label className="flex flex-col items-center justify-center rounded-2xl border-2 border-dashed border-slate-300 bg-white p-8 text-center text-slate-500 cursor-pointer">
            <input
                type="file"
                accept=".xlsx,.xls"
                className="hidden"
                onChange={(event) => {
                    const file = event.target.files?.[0];
                    if (file) {
                        onFileSelected(file);
                    }
                }}
            />
            <span className="text-sm font-medium">拖拽或点击上传问卷星 Excel</span>
            <span className="text-xs text-slate-400">支持 .xlsx / .xls</span>
        </label>
    );
}
