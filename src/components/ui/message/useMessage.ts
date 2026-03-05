import { useContext } from "react";

import { MessageContext } from "@ui/message/MessageProvider";
import { message } from "@ui/message/api";
import type { MessageApi } from "@ui/message/types";

export function useMessage(): MessageApi {
  const context = useContext(MessageContext);
  return context?.api ?? message;
}
