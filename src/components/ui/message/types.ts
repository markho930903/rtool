import type { CSSProperties, ReactNode } from "react";

export type MessageMode = "inline" | "toast";

export type MessageType = "success" | "info" | "warning" | "error" | "loading";

export type MessagePlacement = "top-left" | "top-center" | "top-right" | "bottom-left" | "bottom-center" | "bottom-right";

export type MessageRole = "status" | "alert" | "log";

export interface MessageCommonOptions {
  mode?: MessageMode;
  type?: MessageType;
  title?: ReactNode;
  description?: ReactNode;
  content?: ReactNode;
  icon?: ReactNode;
  showIcon?: boolean;
  closable?: boolean;
  closeLabel?: string;
  className?: string;
  style?: CSSProperties;
  role?: MessageRole;
  actions?: ReactNode;
  placement?: MessagePlacement;
  duration?: number | null;
  pauseOnHover?: boolean;
  priority?: number;
  dedupeKey?: string;
  onClose?: () => void;
}

export interface MessageOpenOptions extends MessageCommonOptions {
  key?: string;
}

export type MessageOpenInput = MessageOpenOptions | ReactNode;

export interface MessageToastItem extends MessageCommonOptions {
  key: string;
  mode: "toast";
  placement: MessagePlacement;
  duration: number | null;
  pauseOnHover: boolean;
  priority: number;
  createdAt: number;
}

export interface MessageRenderProps extends MessageCommonOptions {
  mode?: MessageMode;
  children?: ReactNode;
  onClose?: () => void;
}

export interface MessageApi {
  open: (input: MessageOpenInput) => string;
  update: (key: string, input: MessageOpenInput) => boolean;
  close: (key: string) => void;
  destroy: () => void;
  success: (input: MessageOpenInput) => string;
  info: (input: MessageOpenInput) => string;
  warning: (input: MessageOpenInput) => string;
  error: (input: MessageOpenInput) => string;
  loading: (input: MessageOpenInput) => string;
  promise: <T>(promiseLike: MessagePromiseSource<T>, options?: MessagePromiseOptions<T>) => Promise<T>;
}

export type MessagePromiseSource<T> = Promise<T> | (() => Promise<T>);

export type MessagePromiseStage<TValue> =
  | MessageOpenInput
  | ((value: TValue) => MessageOpenInput)
  | undefined;

export interface MessagePromiseOptions<T> {
  key?: string;
  loading?: MessagePromiseStage<void>;
  success?: MessagePromiseStage<T>;
  error?: MessagePromiseStage<unknown>;
}

export interface MessageProviderConfig {
  maxCount?: number;
  defaultPlacement?: MessagePlacement;
  defaultDuration?: number | null;
  defaultPauseOnHover?: boolean;
  defaultClosable?: boolean;
  defaultCloseLabel?: string;
}
