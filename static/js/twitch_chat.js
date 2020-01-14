function send_message_raw(message) {
        return new Promise(function (resolve, reject) {
            chrome.runtime.sendMessage(null, message, null, function (x) {
                var error = chrome.runtime.lastError;

                if (error != null) {
                    reject(new Error(error.message));

                } else {
                    resolve(x);
                }
            });
        });
    }

    function chrome_port_connect(name) {
        return chrome.runtime.connect(null, { name: name });
    }

let wasm;

const heap = new Array(32);

heap.fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

let heap_next = heap.length;

function dropObject(idx) {
    if (idx < 36) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

let WASM_VECTOR_LEN = 0;

let cachegetUint8Memory0 = null;
function getUint8Memory0() {
    if (cachegetUint8Memory0 === null || cachegetUint8Memory0.buffer !== wasm.memory.buffer) {
        cachegetUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachegetUint8Memory0;
}

let cachedTextEncoder = new TextEncoder('utf-8');

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

function passStringToWasm0(arg, malloc, realloc) {

    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length);
        getUint8Memory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len);

    const mem = getUint8Memory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }

    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3);
        const view = getUint8Memory0().subarray(ptr + offset, ptr + len);
        const ret = encodeString(arg, view);

        offset += ret.written;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachegetInt32Memory0 = null;
function getInt32Memory0() {
    if (cachegetInt32Memory0 === null || cachegetInt32Memory0.buffer !== wasm.memory.buffer) {
        cachegetInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachegetInt32Memory0;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

function getStringFromWasm0(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}
function __wbg_adapter_22(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures__invoke0_mut__h9e036ead768468f1(arg0, arg1);
}

function __wbg_adapter_25(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures__invoke0_mut__h9e036ead768468f1(arg0, arg1);
}

function __wbg_adapter_28(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen__convert__closures__invoke2_mut__h4c88b7aa819369b4(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

function __wbg_adapter_31(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures__invoke1_mut__hbf543a7afe63658a(arg0, arg1, addHeapObject(arg2));
}

function getCachedStringFromWasm0(ptr, len) {
    if (ptr === 0) {
        return getObject(len);
    } else {
        return getStringFromWasm0(ptr, len);
    }
}

function handleError(e) {
    wasm.__wbindgen_exn_store(addHeapObject(e));
}

function init(module) {
    if (typeof module === 'undefined') {
        module = import.meta.url.replace(/\.js$/, '_bg.wasm');
    }
    let result;
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = takeObject(arg0).original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        var ret = false;
        return ret;
    };
    imports.wbg.__widl_f_disconnect_MutationObserver = function(arg0) {
        getObject(arg0).disconnect();
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbg_new_9e4e8c6fadc05c7a = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var v1 = getCachedStringFromWasm0(arg2, arg3);
        var ret = new RegExp(v0, v1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new0_926efe275b9bd771 = function() {
        var ret = new Date();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toUTCString_c6c53dddfae1eb43 = function(arg0) {
        var ret = getObject(arg0).toUTCString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        var ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        var ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__widl_f_log_1_ = function(arg0) {
        console.log(getObject(arg0));
    };
    imports.wbg.__wbg_chromeportconnect_5dc6204b52808a38 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var ret = chrome_port_connect(v0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_onDisconnect_1678887fe804d850 = function(arg0) {
        var ret = getObject(arg0).onDisconnect;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_addListener_2ac5cbd510ccd7c6 = function(arg0, arg1) {
        getObject(arg0).addListener(getObject(arg1));
    };
    imports.wbg.__widl_f_new_MutationObserver = function(arg0) {
        try {
            var ret = new MutationObserver(getObject(arg0));
            return addHeapObject(ret);
        } catch (e) {
            handleError(e);
        }
    };
    imports.wbg.__wbg_new_59cb74e423758ede = function() {
        var ret = new Error();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_stack_558ba5917b466edd = function(arg0, arg1) {
        var ret = getObject(arg1).stack;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbg_error_4bb6c2a97407129a = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
    if (arg0 !== 0) { wasm.__wbindgen_free(arg0, arg1); }
    console.error(v0);
};
imports.wbg.__wbindgen_cb_forget = function(arg0) {
    takeObject(arg0);
};
imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
    var ret = debugString(getObject(arg1));
    var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};
imports.wbg.__wbg_sendmessageraw_06db037e50adb455 = function(arg0, arg1) {
    var v0 = getCachedStringFromWasm0(arg0, arg1);
    var ret = send_message_raw(v0);
    return addHeapObject(ret);
};
imports.wbg.__wbg_then_5a9068d7b674caf9 = function(arg0, arg1, arg2) {
    var ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
};
imports.wbg.__widl_f_clear_timeout_with_handle_Window = function(arg0, arg1) {
    getObject(arg0).clearTimeout(arg1);
};
imports.wbg.__widl_f_set_timeout_with_callback_and_timeout_and_arguments_0_Window = function(arg0, arg1, arg2) {
    try {
        var ret = getObject(arg0).setTimeout(getObject(arg1), arg2);
        return ret;
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_location_Window = function(arg0) {
    var ret = getObject(arg0).location;
    return addHeapObject(ret);
};
imports.wbg.__widl_f_reload_Location = function(arg0) {
    try {
        getObject(arg0).reload();
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_get_NodeList = function(arg0, arg1) {
    var ret = getObject(arg0)[arg1 >>> 0];
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__wbg_replace_8f316f864d6bf31e = function(arg0, arg1, arg2, arg3) {
    var v0 = getCachedStringFromWasm0(arg2, arg3);
    var ret = getObject(arg0).replace(getObject(arg1), v0);
    return addHeapObject(ret);
};
imports.wbg.__widl_f_document_Window = function(arg0) {
    var ret = getObject(arg0).document;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__wbg_length_1881309ca6f2ebd6 = function(arg0) {
    var ret = getObject(arg0).length;
    return ret;
};
imports.wbg.__wbg_get_bf32bf170c3d4d9a = function(arg0, arg1) {
    var ret = getObject(arg0)[arg1 >>> 0];
    return addHeapObject(ret);
};
imports.wbg.__widl_instanceof_MutationRecord = function(arg0) {
    var ret = getObject(arg0) instanceof MutationRecord;
    return ret;
};
imports.wbg.__wbg_now_65115a9aef2f5270 = function() {
    var ret = Date.now();
    return ret;
};
imports.wbg.__wbg_postMessage_9cd8a6302d77f2ce = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    getObject(arg0).postMessage(v0);
};
imports.wbg.__widl_f_query_selector_Document = function(arg0, arg1, arg2) {
    try {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var ret = getObject(arg0).querySelector(v0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_parent_node_Node = function(arg0) {
    var ret = getObject(arg0).parentNode;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__wbg_new_66e20d51c3e33b63 = function() {
    var ret = new Object();
    return addHeapObject(ret);
};
imports.wbg.__wbg_set_c3a2ba27703a6186 = function(arg0, arg1, arg2) {
    try {
        var ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
        return ret;
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_observe_with_options_MutationObserver = function(arg0, arg1, arg2) {
    try {
        getObject(arg0).observe(getObject(arg1), getObject(arg2));
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_type_MutationRecord = function(arg0, arg1) {
    var ret = getObject(arg1).type;
    var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};
imports.wbg.__widl_f_added_nodes_MutationRecord = function(arg0) {
    var ret = getObject(arg0).addedNodes;
    return addHeapObject(ret);
};
imports.wbg.__widl_f_length_NodeList = function(arg0) {
    var ret = getObject(arg0).length;
    return ret;
};
imports.wbg.__widl_f_clone_node_with_deep_Node = function(arg0, arg1) {
    try {
        var ret = getObject(arg0).cloneNode(arg1 !== 0);
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_instanceof_Element = function(arg0) {
    var ret = getObject(arg0) instanceof Element;
    return ret;
};
imports.wbg.__widl_f_first_child_Node = function(arg0) {
    var ret = getObject(arg0).firstChild;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__widl_f_remove_child_Node = function(arg0, arg1) {
    try {
        var ret = getObject(arg0).removeChild(getObject(arg1));
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_query_selector_all_Element = function(arg0, arg1, arg2) {
    try {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var ret = getObject(arg0).querySelectorAll(v0);
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_text_content_Node = function(arg0, arg1) {
    var ret = getObject(arg1).textContent;
    var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};
imports.wbg.__widl_instanceof_HTMLImageElement = function(arg0) {
    var ret = getObject(arg0) instanceof HTMLImageElement;
    return ret;
};
imports.wbg.__widl_f_alt_HTMLImageElement = function(arg0, arg1) {
    var ret = getObject(arg1).alt;
    var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};
imports.wbg.__widl_f_create_text_node_Document = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = getObject(arg0).createTextNode(v0);
    return addHeapObject(ret);
};
imports.wbg.__widl_f_replace_child_Node = function(arg0, arg1, arg2) {
    try {
        var ret = getObject(arg0).replaceChild(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__wbg_test_41b5f603c1c6a281 = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = getObject(arg0).test(v0);
    return ret;
};
imports.wbg.__wbg_setlastindex_636403a6b8935149 = function(arg0, arg1) {
    getObject(arg0).lastIndex = arg1 >>> 0;
};
imports.wbg.__wbg_exec_641c92568d076518 = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = getObject(arg0).exec(v0);
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__wbg_removeListener_f1deaca333139c3d = function(arg0, arg1) {
    getObject(arg0).removeListener(getObject(arg1));
};
imports.wbg.__wbindgen_throw = function(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};
imports.wbg.__wbindgen_rethrow = function(arg0) {
    throw takeObject(arg0);
};
imports.wbg.__wbg_then_79de0b6809569306 = function(arg0, arg1) {
    var ret = getObject(arg0).then(getObject(arg1));
    return addHeapObject(ret);
};
imports.wbg.__wbg_resolve_4e9c46f7e8321315 = function(arg0) {
    var ret = Promise.resolve(getObject(arg0));
    return addHeapObject(ret);
};
imports.wbg.__wbg_globalThis_1c2aa6db3ecb073e = function() {
    try {
        var ret = globalThis.globalThis;
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__wbg_self_e5cdcdef79894248 = function() {
    try {
        var ret = self.self;
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__wbg_window_44ec8ac43884a4cf = function() {
    try {
        var ret = window.window;
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__wbg_global_c9abcb94a14733fe = function() {
    try {
        var ret = global.global;
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__wbindgen_is_undefined = function(arg0) {
    var ret = getObject(arg0) === undefined;
    return ret;
};
imports.wbg.__wbg_newnoargs_a9cd98b36c38f53e = function(arg0, arg1) {
    var v0 = getCachedStringFromWasm0(arg0, arg1);
    var ret = new Function(v0);
    return addHeapObject(ret);
};
imports.wbg.__wbg_call_222be890f6f564bb = function(arg0, arg1) {
    try {
        var ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
    var ret = getObject(arg0);
    return addHeapObject(ret);
};
imports.wbg.__widl_instanceof_Window = function(arg0) {
    var ret = getObject(arg0) instanceof Window;
    return ret;
};
imports.wbg.__wbindgen_closure_wrapper98 = function(arg0, arg1, arg2) {

    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = () => {
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return __wbg_adapter_25(a, state.b, );
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(33)(a, state.b);
            else state.a = a;
        }
    }
    ;
    real.original = state;
    var ret = real;
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper97 = function(arg0, arg1, arg2) {

    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = () => {
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return __wbg_adapter_22(a, state.b, );
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(33)(a, state.b);
            else state.a = a;
        }
    }
    ;
    real.original = state;
    var ret = real;
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper100 = function(arg0, arg1, arg2) {

    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (arg0, arg1) => {
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return __wbg_adapter_28(a, state.b, arg0, arg1);
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(33)(a, state.b);
            else state.a = a;
        }
    }
    ;
    real.original = state;
    var ret = real;
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper353 = function(arg0, arg1, arg2) {

    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (arg0) => {
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return __wbg_adapter_31(a, state.b, arg0);
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(33)(a, state.b);
            else state.a = a;
        }
    }
    ;
    real.original = state;
    var ret = real;
    return addHeapObject(ret);
};

if ((typeof URL === 'function' && module instanceof URL) || typeof module === 'string' || (typeof Request === 'function' && module instanceof Request)) {

    const response = fetch(module);
    if (typeof WebAssembly.instantiateStreaming === 'function') {
        result = WebAssembly.instantiateStreaming(response, imports)
        .catch(e => {
            return response
            .then(r => {
                if (r.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);
                    return r.arrayBuffer();
                } else {
                    throw e;
                }
            })
            .then(bytes => WebAssembly.instantiate(bytes, imports));
        });
    } else {
        result = response
        .then(r => r.arrayBuffer())
        .then(bytes => WebAssembly.instantiate(bytes, imports));
    }
} else {

    result = WebAssembly.instantiate(module, imports)
    .then(result => {
        if (result instanceof WebAssembly.Instance) {
            return { instance: result, module };
        } else {
            return result;
        }
    });
}
return result.then(({instance, module}) => {
    wasm = instance.exports;
    init.__wbindgen_wasm_module = module;
    wasm.__wbindgen_start();
    return wasm;
});
}

init(chrome.runtime.getURL("js/twitch-chat.wasm")).catch(console.error);
//# sourceMappingURL=twitch_chat.js.map
