/* @ts-self-types="./vastlint_wasm.d.ts" */

import * as wasm from "./vastlint_wasm_bg.wasm";
import { __wbg_set_wasm } from "./vastlint_wasm_bg.js";
__wbg_set_wasm(wasm);
if (typeof wasm.__wbindgen_start === "function") wasm.__wbindgen_start();
export {
    rules, validate, validateWithOptions
} from "./vastlint_wasm_bg.js";
