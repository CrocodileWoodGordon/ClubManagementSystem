import { ExcelDropzone } from "@/components/upload/ExcelDropzone";
import { SectionCard } from "@/components/common/SectionCard";
import { StudentTable } from "@/components/tables/StudentTable";

const MOCK_STUDENTS = [
    { id: "1", name: "张三", originalClass: "三年二班", status: "待分班" },
    { id: "2", name: "李四", originalClass: "三年三班", status: "待分班" },
];

export default function EnrollmentPage() {
    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">报名管理</h1>
            <SectionCard title="导入问卷星报名" description="Excel 列结构固定，系统会自动识别">
                <ExcelDropzone onFileSelected={() => {}} />
            </SectionCard>
            <SectionCard title="待处理名单" description="解析失败或班级未确定的学生">
                <StudentTable students={MOCK_STUDENTS} />
            </SectionCard>
        </div>
    );
}
