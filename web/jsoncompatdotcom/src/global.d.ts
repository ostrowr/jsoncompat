declare module "/jsoncompat_wasm/jsoncompat_wasm.js" {
  const init: () => Promise<void>;
  export default init;

  export function check_compat(
    oldSchema: string,
    newSchema: string,
    role: string
  ): Promise<boolean>;

  export function generate_value(schema: string, depth: number): Promise<string>;
}

declare module "*/jsoncompat_wasm.js" {
  const init: () => Promise<void>;
  export default init;
  export function check_compat(
    oldSchema: string,
    newSchema: string,
    role: string
  ): Promise<boolean>;
  export function generate_value(schema: string, depth: number): Promise<string>;
}
