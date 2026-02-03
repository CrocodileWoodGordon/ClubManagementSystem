import Link from "next/link";

export default function Home() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-slate-50">
      <main className="space-y-6 rounded-3xl border border-slate-200 bg-white p-12 text-center shadow-xl">
        <h1 className="text-3xl font-semibold text-slate-900">
          社团管理系统
        </h1>
        <p className="text-slate-600">
          开始进行报名导入、分班、考勤与费用结算，请进入后台面板。
        </p>
        <Link
          href="/dashboard"
          className="inline-flex items-center justify-center rounded-2xl bg-slate-900 px-6 py-3 text-base font-medium text-white"
        >
          进入后台
        </Link>
      </main>
    </div>
  );
}
