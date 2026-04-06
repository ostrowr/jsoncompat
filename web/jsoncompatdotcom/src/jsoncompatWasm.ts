import type * as JsoncompatModule from "jsoncompat";

export type JsoncompatGenerator = {
	generate_value(depth: number): string;
};

type JsoncompatWasmModule = typeof JsoncompatModule & {
	generator_for?: (schema: string) => JsoncompatGenerator;
};

export function generatorFor(
	module: typeof JsoncompatModule,
	schema: string,
): JsoncompatGenerator {
	const wasm = module as JsoncompatWasmModule;
	return (
		wasm.generator_for?.(schema) ?? {
			generate_value: (depth) => wasm.generate_value(schema, depth),
		}
	);
}
