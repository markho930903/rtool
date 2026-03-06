import { useAppManagerScreen } from "./useAppManagerScreen";

export const useAppManagerController = useAppManagerScreen;

export type AppManagerController = ReturnType<typeof useAppManagerScreen>;
