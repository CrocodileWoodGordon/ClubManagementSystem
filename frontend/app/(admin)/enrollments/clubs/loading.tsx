import { SectionCard } from "@/components/common/SectionCard";

export default function EnrollmentClubsLoading() {
    return (
        <div className="space-y-8">
            <SectionCard title="社团列表">
                <p className="text-sm text-slate-500">正在加载社团数据...</p>
            </SectionCard>
            <SectionCard title="社团成员管理">
                <p className="text-sm text-slate-500">请稍候，正在准备页面。</p>
            </SectionCard>
        </div>
    );
}
