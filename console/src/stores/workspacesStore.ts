import { create } from 'zustand';

type WorkspacesState = {
  workspaces: any[];
  setWorkspaces: (workspaces: any[]) => void;
  addWorkspace: (workspace: any) => void;
  getWorkspace: (id: string) => any;
};

const useWorkspacesStore = create<WorkspacesState>((set, get) => ({
  workspaces: [],
  getWorkspace: (id) => get().workspaces.filter((workspace) => workspace.id === parseInt(id))[0],
  setWorkspaces: (workspaces) => set({ workspaces }),
  addWorkspace: (workspace) => set({ workspaces: [...get().workspaces, workspace] }),
}));

export const selector = (store: WorkspacesState) => ({
  workspaces: store.workspaces,
  setWorkspaces: store.setWorkspaces,
  addWorkspace: store.addWorkspace,
  getWorkspace: store.getWorkspace,
});

export default useWorkspacesStore;
