interface StudentTableProps {
    students: Array<{
        id: string;
        name: string;
        originalClass: string;
        status?: string;
    }>;
}

export function StudentTable({ students }: StudentTableProps) {
    return (
        <div className="overflow-x-auto">
            <table className="min-w-full border border-slate-200 rounded-xl">
                <thead className="bg-slate-50 text-left text-sm text-slate-500">
                    <tr>
                        <th className="px-4 py-2">姓名</th>
                        <th className="px-4 py-2">原班级</th>
                        <th className="px-4 py-2">状态</th>
                    </tr>
                </thead>
                <tbody>
                    {students.map((student) => (
                        <tr key={student.id} className="border-t">
                            <td className="px-4 py-2 font-medium text-slate-900">{student.name}</td>
                            <td className="px-4 py-2 text-slate-600">{student.originalClass}</td>
                            <td className="px-4 py-2 text-slate-600">{student.status ?? "--"}</td>
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}
