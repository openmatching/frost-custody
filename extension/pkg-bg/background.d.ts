declare namespace wasm_bindgen {
	/* tslint:disable */
	/* eslint-disable */
	/**
	 * Initialize background service worker
	 * Called from JavaScript glue via wasm_bindgen
	 */
	export function init_background(): void;
	/**
	 * Handle extension icon click
	 * Returns the window ID that should open the side panel
	 */
	export function handle_icon_click(window_id: number): number;
	/**
	 * Handle extension installation
	 */
	export function handle_install(): void;
	/**
	 * Handle message from main extension
	 * This enables communication between UI and background
	 */
	export function handle_message(message: any): any;
	
}

declare type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

declare interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly init_background: () => void;
  readonly handle_icon_click: (a: number) => number;
  readonly handle_install: () => void;
  readonly handle_message: (a: any) => any;
  readonly main: (a: number, b: number) => number;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __wbindgen_start: () => void;
}

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
declare function wasm_bindgen (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
