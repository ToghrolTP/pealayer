export interface WorkspaceSummary {
  id: string; name: string; icon: string; order: number; mode: 'simple' | 'nle';
}

export function selectedWorkspace(profiles: readonly WorkspaceSummary[] | undefined, id: string | null | undefined): WorkspaceSummary | undefined {
  return id == null ? undefined : profiles?.find((profile) => profile.id === id);
}
