import { SectionCard } from "@/components/common/SectionCard";

export default function AttendancePage() {
    return (
        <div className="space-y-8">
            <h1 className="text-2xl font-semibold text-slate-900">考勤管理</h1>
            <SectionCard title="生成考勤表" description="每个班级生成 PDF/Excel 模板提供老师线下录入">
                <p className="text-sm text-slate-600">
                    选择班级后触发后端任务生成空表，可批量下载。
                </p>
            </SectionCard>
            <SectionCard title="导入考勤结果" description="期末回收纸质表后统一 Excel 导入">
                <p className="text-sm text-slate-600">未来将支持批量拖拽文件并展示解析状态。</p>
            </SectionCard>
        </div>
    );
}
