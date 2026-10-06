import { useEffect, useState } from "react";
import { useParams, Link } from "react-router-dom";
import { useAuth } from "@/context/auth";
import { api } from "@/api/client";
import type {
  OrgDetail,
  OrgChild,
  AdminGroupRow,
  AdminSubmissionRow,
  AdminUserRow,
} from "@/api/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Separator } from "@/components/ui/separator";
import { sourceTypeLabel, statusLabel, statusVariant } from "@/lib/labels";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Building2, Users, FileText, GitBranch } from "lucide-react";

function orgLabel(org: OrgDetail): string {
  return org.short_name;
}

function OrgHeader({ org }: { org: OrgDetail }) {
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-3">
        <Building2 className="h-6 w-6 text-primary" />
        <h1 className="text-2xl font-bold">{orgLabel(org)}</h1>
        <Badge variant={org.is_active ? "default" : "secondary"}>
          {org.is_active ? "Активний" : "Неактивний"}
        </Badge>
      </div>
      {org.full_name && (
        <p className="text-sm text-muted-foreground">{org.full_name}</p>
      )}
      {org.kind && (
        <p className="text-xs text-muted-foreground">Тип: {org.kind}</p>
      )}
    </div>
  );
}

function ChildrenSection({ children }: { children: OrgChild[] }) {
  if (children.length === 0) return null;
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <GitBranch className="h-4 w-4" />
          Підпорядковані підрозділи ({children.length})
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex flex-wrap gap-2">
          {children.map((c) => (
            <Link key={c.id} to={`/org/${c.id}`}>
              <Badge
                variant="outline"
                className="cursor-pointer transition-colors hover:bg-primary/10 hover:text-primary"
              >
                {c.label}
              </Badge>
            </Link>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function GroupsTable({ groups }: { groups: AdminGroupRow[] }) {
  if (groups.length === 0) {
    return <p className="py-8 text-center text-sm text-muted-foreground">Груп підготовки немає</p>;
  }
  return (
    <div className="overflow-x-auto">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Підрозділ</TableHead>
            <TableHead>Вид</TableHead>
            <TableHead>ВОС</TableHead>
            <TableHead>Полігон</TableHead>
            <TableHead>Початок</TableHead>
            <TableHead>Кінець</TableHead>
            <TableHead className="text-right">План</TableHead>
            <TableHead className="text-right">Прибули</TableHead>
            <TableHead className="text-right">Навч.</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {groups.map((g) => (
            <TableRow key={g.id}>
              <TableCell className="font-medium">{g.org_label}</TableCell>
              <TableCell>{g.training_kind}</TableCell>
              <TableCell>{g.vos_label}</TableCell>
              <TableCell>{g.site_label}</TableCell>
              <TableCell>{g.planned_start}</TableCell>
              <TableCell>{g.planned_end}</TableCell>
              <TableCell className="text-right">{g.planned_count}</TableCell>
              <TableCell className="text-right">{g.arrived_count}</TableCell>
              <TableCell className="text-right">{g.in_training_count}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

function SubmissionsTable({ submissions }: { submissions: AdminSubmissionRow[] }) {
  if (submissions.length === 0) {
    return <p className="py-8 text-center text-sm text-muted-foreground">Подань немає</p>;
  }
  return (
    <div className="overflow-x-auto">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-[30%]">Підрозділ</TableHead>
            <TableHead className="w-[12%]">Тип</TableHead>
            <TableHead className="w-[14%]">Статус</TableHead>
            <TableHead className="w-[16%] tabular-nums">Станом на</TableHead>
            <TableHead className="w-[28%] tabular-nums">Оновлено</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {submissions.map((s) => (
            <TableRow key={s.id}>
              <TableCell className="font-medium">{s.org_label}</TableCell>
              <TableCell>{sourceTypeLabel(s.source_type)}</TableCell>
              <TableCell>
                <Badge variant={statusVariant(s.status)}>
                  {statusLabel(s.status)}
                </Badge>
              </TableCell>
              <TableCell className="tabular-nums">{s.as_of_date}</TableCell>
              <TableCell className="tabular-nums">{s.updated_at}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

function UsersTable({ users }: { users: AdminUserRow[] }) {
  if (users.length === 0) {
    return <p className="py-8 text-center text-sm text-muted-foreground">Користувачів немає</p>;
  }
  return (
    <div className="overflow-x-auto">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Логін</TableHead>
            <TableHead>Ім'я</TableHead>
            <TableHead>Ролі</TableHead>
            <TableHead>Статус</TableHead>
            <TableHead>Створено</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {users.map((u) => (
            <TableRow key={u.id}>
              <TableCell className="font-medium">{u.login}</TableCell>
              <TableCell>{u.display_name ?? "—"}</TableCell>
              <TableCell>
                <div className="flex flex-wrap gap-1">
                  {u.roles.map((r, i) => (
                    <Badge key={i} variant="outline" className="text-xs">
                      {r.role} @ {r.org_label}
                    </Badge>
                  ))}
                </div>
              </TableCell>
              <TableCell>
                <Badge variant={u.is_active ? "default" : "destructive"}>
                  {u.is_active ? "Активний" : "Неактивний"}
                </Badge>
                {u.must_change_password && (
                  <Badge variant="secondary" className="ml-1 text-xs">
                    temp
                  </Badge>
                )}
              </TableCell>
              <TableCell>{u.created_at}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}

export function OrgDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { isAdmin } = useAuth();
  const [org, setOrg] = useState<OrgDetail | null>(null);
  const [children, setChildren] = useState<OrgChild[]>([]);
  const [groups, setGroups] = useState<AdminGroupRow[]>([]);
  const [submissions, setSubmissions] = useState<AdminSubmissionRow[]>([]);
  const [users, setUsers] = useState<AdminUserRow[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    const orgId = parseInt(id, 10);

    Promise.allSettled([
      api.get<OrgDetail>(`/orgs/${orgId}`),
      api.get<OrgChild[]>(`/orgs/${orgId}/children`),
      api.get<AdminGroupRow[]>(`/orgs/${orgId}/groups`),
      api.get<AdminSubmissionRow[]>(`/orgs/${orgId}/submissions`),
      isAdmin ? api.get<AdminUserRow[]>(`/orgs/${orgId}/users`) : Promise.resolve([]),
    ])
      .then(([orgRes, childrenRes, groupsRes, subsRes, usersRes]) => {
        if (orgRes.status === "fulfilled") setOrg(orgRes.value);
        if (childrenRes.status === "fulfilled") setChildren(childrenRes.value);
        if (groupsRes.status === "fulfilled") setGroups(groupsRes.value);
        if (subsRes.status === "fulfilled") setSubmissions(subsRes.value);
        if (usersRes.status === "fulfilled") setUsers(usersRes.value);
      })
      .finally(() => setLoading(false));
  }, [id, isAdmin]);

  if (loading) {
    return (
      <div className="flex flex-col gap-4">
        <Skeleton className="h-10 w-64" />
        <Skeleton className="h-48 w-full" />
        <Skeleton className="h-48 w-full" />
      </div>
    );
  }

  if (!org) {
    return <p className="py-8 text-center text-muted-foreground">Підрозділ не знайдено</p>;
  }

  return (
    <div className="flex flex-col gap-6">
      <OrgHeader org={org} />
      <Separator />
      <ChildrenSection children={children} />

      <Tabs defaultValue="groups">
        <TabsList>
          <TabsTrigger value="groups" className="gap-1 sm:gap-1.5">
            <Users className="h-4 w-4" />
            <span className="hidden sm:inline">Групи</span>
            ({groups.length})
          </TabsTrigger>
          <TabsTrigger value="submissions" className="gap-1 sm:gap-1.5">
            <FileText className="h-4 w-4" />
            <span className="hidden sm:inline">Подання</span>
            ({submissions.length})
          </TabsTrigger>
          {isAdmin && (
            <TabsTrigger value="users" className="gap-1 sm:gap-1.5">
              <Users className="h-4 w-4" />
              <span className="hidden sm:inline">Користувачі</span>
              ({users.length})
            </TabsTrigger>
          )}
        </TabsList>

        <TabsContent value="groups">
          <Card>
            <CardContent className="pt-6">
              <GroupsTable groups={groups} />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="submissions">
          <Card>
            <CardContent className="pt-6">
              <SubmissionsTable submissions={submissions} />
            </CardContent>
          </Card>
        </TabsContent>

        {isAdmin && (
          <TabsContent value="users">
            <Card>
              <CardContent className="pt-6">
                <UsersTable users={users} />
              </CardContent>
            </Card>
          </TabsContent>
        )}
      </Tabs>
    </div>
  );
}
