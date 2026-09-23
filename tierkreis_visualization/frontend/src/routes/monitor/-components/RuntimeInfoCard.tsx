import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useRuntimeInfoQuery } from "@/data/api";

export const RuntimeInfoCard = () => {
  const { data, isLoading } = useRuntimeInfoQuery();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Runtime</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading && <div className="text-muted-foreground">Loading...</div>}
        {data && (
          <div className="flex flex-col gap-2">
            <div className="text-sm text-muted-foreground">
              tierkreis v{data.version}
            </div>
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left border-b">
                  <th className="py-1">Executor</th>
                  <th className="py-1">Kind</th>
                  <th className="py-1">Workers</th>
                  <th className="py-1">Resources</th>
                </tr>
              </thead>
              <tbody>
                {data.executors.map((executor) => (
                  <tr key={executor.name} className="border-b">
                    <td className="py-1">{executor.name}</td>
                    <td className="py-1">{executor.kind}</td>
                    <td className="py-1">{executor.worker_count}</td>
                    <td className="py-1">
                      {Object.entries(executor.details)
                        .map(([key, value]) => `${key}=${value}`)
                        .join(", ") || "-"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </CardContent>
    </Card>
  );
};
