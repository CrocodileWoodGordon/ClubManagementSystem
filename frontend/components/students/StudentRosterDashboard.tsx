"use client";

import { useState } from "react";

import { SectionCard } from "@/components/common/SectionCard";
import { StudentImportPanel } from "@/components/upload/StudentImportPanel";
import { TeacherChildImportPanel } from "@/components/upload/TeacherChildImportPanel";

import {
    CampusOption,
    StudentRosterWorkspace,
    TermOption,
} from "./StudentRosterWorkspace";

interface StudentRosterDashboardProps {
    terms: TermOption[];
    campuses: CampusOption[];
    defaultTermId?: string;
}

export function StudentRosterDashboard({
    terms,
    campuses,
    defaultTermId,
}: StudentRosterDashboardProps) {
    const [refreshToken, setRefreshToken] = useState(0);

    const handleImportCompleted = () => {
        setRefreshToken((token) => token + 1);
    };

    return (
        <div className="space-y-8">
            <SectionCard
                title="导入学生名单"
                description="上传“校区/班级/姓名” Excel，一次性建立学生与班级关联。"
            >
                <StudentImportPanel onCompleted={handleImportCompleted} />
            </SectionCard>
            <SectionCard
                title="教师子女批量标记"
                description="按学期、校区导入 Excel，将名单中的学生批量标记为教师子女。"
            >
                <TeacherChildImportPanel
                    terms={terms}
                    campuses={campuses}
                    defaultTermId={defaultTermId}
                    onCompleted={handleImportCompleted}
                />
            </SectionCard>
            <SectionCard
                title="学生名册工作台"
                description="按学期和校区筛选班级、维护班主任信息，并支持单个学生的增删改。"
            >
                <StudentRosterWorkspace
                    terms={terms}
                    campuses={campuses}
                    defaultTermId={defaultTermId}
                    refreshToken={refreshToken}
                />
            </SectionCard>
        </div>
    );
}
