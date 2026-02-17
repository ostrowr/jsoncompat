const TYPE_COLOR = {
  str: 0x60a5fa,
  int: 0xa78bfa,
  list: 0xfacc15,
  neutral: 0xc8d4e8,
  fallback: 0x60a5fa,
} as const;

const normalized = (displayType: string): string => {
  const lowered = displayType.toLowerCase();
  if (lowered.startsWith("list[")) {
    return "list";
  }
  if (lowered.includes("int")) {
    return "int";
  }
  if (lowered.includes("number")) {
    return "int";
  }
  if (lowered.includes("bool")) {
    return "int";
  }
  if (lowered.includes("str")) {
    return "str";
  }
  if (lowered.includes("object")) {
    return "neutral";
  }
  if (lowered.includes("null")) {
    return "neutral";
  }
  return "fallback";
};

export const colorForDisplayType = (displayType: string): number => {
  const key = normalized(displayType);
  return TYPE_COLOR[key as keyof typeof TYPE_COLOR] ?? TYPE_COLOR.fallback;
};
