import { SectionCard } from "@/components/common/SectionCard";
import { StudentTable } from "@/components/tables/StudentTable";

const MOCK_STUDENTS = [
    { id: "1", name: "张三", originalClass: "三年一班", status: "机器人 / 周一" },
    { id: "2", name: "王五", originalClass: "四年三班", status: "创客 / 周五" },
];

export default function StudentsPage() {
    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">学生名册</h1>
            <SectionCard title="全量学生库">
                <StudentTable students={MOCK_STUDENTS} />
            </SectionCard>
        </div>
    );
}
