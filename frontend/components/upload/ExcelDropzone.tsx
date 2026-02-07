"use client";

interface ExcelDropzoneProps {
    onFileSelected: (file: File) => void;
    disabled?: boolean;
}

export function ExcelDropzone({ onFileSelected, disabled = false }: ExcelDropzoneProps) {
    return (
        <label
            className={[
                "flex flex-col items-center justify-center rounded-2xl border-2 border-dashed bg-white p-8 text-center text-slate-500",
                disabled ? "cursor-not-allowed opacity-60" : "cursor-pointer hover:border-slate-400",
                "border-slate-300 transition",
            ].join(" ")}
        >
            <input
                type="file"
                accept=".xlsx,.xls"
                className="hidden"
                disabled={disabled}
                onChange={(event) => {
                    const file = event.target.files?.[0];
                    event.target.value = "";
                    if (file && !disabled) {
                        onFileSelected(file);
                    }
                }}
            />
            <span className="text-sm font-medium">
                {disabled ? "处理中..." : "拖拽或点击上传问卷星 Excel"}
            </span>
            <span className="text-xs text-slate-400">支持 .xlsx / .xls</span>
        </label>
    );
}
