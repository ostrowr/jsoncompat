const TYPE_COLOR = {
  str: 0x60a5fa,
  int: 0x8b5cf6,
  bool: 0xf59e0b,
  list: 0xeab308,
  number: 0x7c83fd,
  object: 0x64748b,
  null: 0x94a3b8,
  fallback: 0x60a5fa,
} as const;

const normalized = (displayType: string): string => {
  const lowered = displayType.toLowerCase();
  if (lowered.startsWith("list[")) {
    return "list";
  }
  if (lowered.includes("bool")) {
    return "bool";
  }
  if (lowered.includes("int")) {
    return "int";
  }
  if (lowered.includes("number")) {
    return "number";
  }
  if (lowered.includes("str")) {
    return "str";
  }
  if (lowered.includes("object")) {
    return "object";
  }
  if (lowered.includes("null")) {
    return "null";
  }
  return "fallback";
};

export const colorForDisplayType = (displayType: string): number => {
  const key = normalized(displayType);
  return TYPE_COLOR[key as keyof typeof TYPE_COLOR] ?? TYPE_COLOR.fallback;
};
