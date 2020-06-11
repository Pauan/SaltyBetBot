// TODO cancellation
    function fetch_(url) {
        // TODO cache ?
        // TODO integrity ?
        return fetch(chrome.runtime.getURL(url), {
            credentials: "same-origin",
            mode: "same-origin"
        // TODO check HTTP status codes ?
        }).then(function (response) {
            return response.text();
        });
    }

    function remove_tabs(tabs) {
        // TODO move this into Rust ?
        var ids = tabs.map(function (tab) { return tab.id; });

        return new Promise(function (resolve, reject) {
            chrome.tabs.remove(ids, function () {
                if (chrome.runtime.lastError != null) {
                    reject(new Error(chrome.runtime.lastError.message));

                } else {
                    resolve();
                }
            });
        });
    }

function chrome_on_message() {
        return chrome.runtime.onMessage;
    }

    function chrome_on_connect() {
        return chrome.runtime.onConnect;
    }

let wasm;

const heap = new Array(32).fill(undefined);

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

function isLikeNone(x) {
    return x === undefined || x === null;
}

let cachegetFloat64Memory0 = null;
function getFloat64Memory0() {
    if (cachegetFloat64Memory0 === null || cachegetFloat64Memory0.buffer !== wasm.memory.buffer) {
        cachegetFloat64Memory0 = new Float64Array(wasm.memory.buffer);
    }
    return cachegetFloat64Memory0;
}

let cachegetInt32Memory0 = null;
function getInt32Memory0() {
    if (cachegetInt32Memory0 === null || cachegetInt32Memory0.buffer !== wasm.memory.buffer) {
        cachegetInt32Memory0 = new Int32Array(wasm.memory.buffer);
    }
    return cachegetInt32Memory0;
}

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });

cachedTextDecoder.decode();

let cachegetUint8Memory0 = null;
function getUint8Memory0() {
    if (cachegetUint8Memory0 === null || cachegetUint8Memory0.buffer !== wasm.memory.buffer) {
        cachegetUint8Memory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachegetUint8Memory0;
}

function getStringFromWasm0(ptr, len) {
    return cachedTextDecoder.decode(getUint8Memory0().subarray(ptr, ptr + len));
}

let WASM_VECTOR_LEN = 0;

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

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {
        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(dtor)(a, state.b);
            else state.a = a;
        }
    };
    real.original = state;
    return real;
}
function __wbg_adapter_26(arg0, arg1, arg2, arg3) {
    var ptr0 = passStringToWasm0(arg2, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    wasm.wasm_bindgen__convert__closures__invoke2_mut__h748a51c5be8ac410(arg0, arg1, ptr0, len0, addHeapObject(arg3));
}

function __wbg_adapter_29(arg0, arg1, arg2, arg3, arg4) {
    var ptr0 = passStringToWasm0(arg2, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    var ret = wasm.wasm_bindgen__convert__closures__invoke3_mut__h36ffe9a78d1e1b8c(arg0, arg1, ptr0, len0, addHeapObject(arg3), addHeapObject(arg4));
    return ret !== 0;
}

let stack_pointer = 32;

function addBorrowedObject(obj) {
    if (stack_pointer == 1) throw new Error('out of js stack');
    heap[--stack_pointer] = obj;
    return stack_pointer;
}
function __wbg_adapter_32(arg0, arg1, arg2) {
    try {
        wasm.wasm_bindgen__convert__closures__invoke1_mut_ref__h05bc548847ae5297(arg0, arg1, addBorrowedObject(arg2));
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

function __wbg_adapter_35(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures__invoke0_mut__h15d2b3e903a521ad(arg0, arg1);
}

function __wbg_adapter_38(arg0, arg1, arg2) {
    try {
        wasm.wasm_bindgen__convert__closures__invoke1_mut_ref__h05bc548847ae5297(arg0, arg1, addBorrowedObject(arg2));
    } finally {
        heap[stack_pointer++] = undefined;
    }
}

function __wbg_adapter_41(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures__invoke1_mut__h15df99cec74c701d(arg0, arg1, addHeapObject(arg2));
}

function getCachedStringFromWasm0(ptr, len) {
    if (ptr === 0) {
        return getObject(len);
    } else {
        return getStringFromWasm0(ptr, len);
    }
}

function handleError(f) {
    return function () {
        try {
            return f.apply(this, arguments);

        } catch (e) {
            wasm.__wbindgen_exn_store(addHeapObject(e));
        }
    };
}

async function load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);

    } else {

        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };

        } else {
            return instance;
        }
    }
}

async function init(input) {
    if (typeof input === 'undefined') {
        input = import.meta.url.replace(/\.js$/, '_bg.wasm');
    }
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg_performance_781c00e4226de6c4 = function(arg0) {
        var ret = getObject(arg0).performance;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_now_43100dbb52857cc6 = function(arg0) {
        var ret = getObject(arg0).now();
        return ret;
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbg_error_20ef23c2407793d3 = function(arg0) {
        console.error(getObject(arg0));
    };
    imports.wbg.__wbg_instanceof_Error_acc6f92bc405b9b4 = function(arg0) {
        var ret = getObject(arg0) instanceof Error;
        return ret;
    };
    imports.wbg.__wbg_message_c1b5fb9ff275d024 = function(arg0) {
        var ret = getObject(arg0).message;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = takeObject(arg0).original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        var ret = false;
        return ret;
    };
    imports.wbg.__wbg_removeListener_f1deaca333139c3d = function(arg0, arg1) {
        getObject(arg0).removeListener(getObject(arg1));
    };
    imports.wbg.__wbg_key_dc96bc911c3c4f58 = function(arg0) {
        var ret = getObject(arg0).key;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        var ret = typeof(obj) === 'number' ? obj : undefined;
        getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbg_value_1d0cf1013d34a19b = function(arg0) {
        var ret = getObject(arg0).value;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_lowerBound_ab2a409f8ed2f113 = handleError(function(arg0, arg1) {
        var ret = IDBKeyRange.lowerBound(getObject(arg0), arg1 !== 0);
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_objectStore_005ba43ca5186094 = handleError(function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var ret = getObject(arg0).objectStore(v0);
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_getAll_96c55559c6a48d26 = handleError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).getAll(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        var ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_setonsuccess_a57c128411b67393 = function(arg0, arg1) {
        getObject(arg0).onsuccess = getObject(arg1);
    };
    imports.wbg.__wbg_setonerror_ec6122177917195b = function(arg0, arg1) {
        getObject(arg0).onerror = getObject(arg1);
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        var ret = getObject(arg0) === undefined;
        return ret;
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        var ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_log_eb1108411ecc4a7f = function(arg0) {
        console.log(getObject(arg0));
    };
    imports.wbg.__wbg_clear_f89c9014923faf1f = handleError(function(arg0) {
        var ret = getObject(arg0).clear();
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_call_0246f1c8ff252fb6 = handleError(function(arg0, arg1, arg2) {
        var ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_new_d6227c3c833572bb = function() {
        var ret = new Object();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_set_6a666216929b0387 = handleError(function(arg0, arg1, arg2) {
        var ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
        return ret;
    });
    imports.wbg.__wbg_createObjectStore_896f4771da0b9f09 = handleError(function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var ret = getObject(arg0).createObjectStore(v0, getObject(arg3));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_openCursor_2e49caa15a9fe2d4 = handleError(function(arg0) {
        var ret = getObject(arg0).openCursor();
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_new0_210d1cef1e5ccdd4 = function() {
        var ret = new Date();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toUTCString_26e6acb7f2c1fdfe = function(arg0) {
        var ret = getObject(arg0).toUTCString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_deleteObjectStore_75593c43f711d953 = handleError(function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        getObject(arg0).deleteObjectStore(v0);
    });
    imports.wbg.__wbg_chromeonconnect_5ed2be58fdccfdfa = function() {
        var ret = chrome_on_connect();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_addListener_2ac5cbd510ccd7c6 = function(arg0, arg1) {
        getObject(arg0).addListener(getObject(arg1));
    };
    imports.wbg.__wbg_chromeonmessage_ce55601db77c298f = function() {
        var ret = chrome_on_message();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_indexedDB_9b799707d734eced = handleError(function(arg0) {
        var ret = getObject(arg0).indexedDB;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_open_1ed105188cf7301b = handleError(function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var ret = getObject(arg0).open(v0, arg3 >>> 0);
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_setonupgradeneeded_52d989389ab8788f = function(arg0, arg1) {
        getObject(arg0).onupgradeneeded = getObject(arg1);
    };
    imports.wbg.__wbg_setonblocked_3f01ad8778c90548 = function(arg0, arg1) {
        getObject(arg0).onblocked = getObject(arg1);
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        var ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_delete_884219abc2d48a3d = handleError(function(arg0, arg1) {
        var ret = getObject(arg0).delete(getObject(arg1));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_fetch_4c5410c19d844163 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var ret = fetch_(v0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_close_5aa2f5abcfd4a8e5 = function(arg0) {
        getObject(arg0).close();
    };
    imports.wbg.__wbg_disconnect_9b6a4e5b14a30a8f = function(arg0) {
        getObject(arg0).disconnect();
    };
    imports.wbg.__wbg_onDisconnect_1678887fe804d850 = function(arg0) {
        var ret = getObject(arg0).onDisconnect;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_name_c7fb65250233ceef = function(arg0, arg1) {
        var ret = getObject(arg1).name;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__wbg_sender_da7503d2b86e62be = function(arg0) {
        var ret = getObject(arg0).sender;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_tab_c51db073e6629cb2 = function(arg0) {
        var ret = getObject(arg0).tab;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_onMessage_39105b92abbad6eb = function(arg0) {
        var ret = getObject(arg0).onMessage;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_0d50725e1ae68303 = function() {
        var ret = new Array();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_push_46274b393147c746 = function(arg0, arg1) {
        var ret = getObject(arg0).push(getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_removetabs_ee935c04f8aa5977 = function(arg0) {
        var ret = remove_tabs(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_postMessage_9cd8a6302d77f2ce = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        getObject(arg0).postMessage(v0);
    };
    imports.wbg.__wbg_add_2657a3067e16cb15 = handleError(function(arg0, arg1) {
        var ret = getObject(arg0).add(getObject(arg1));
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_result_f103d6c95969477c = handleError(function(arg0) {
        var ret = getObject(arg0).result;
        return addHeapObject(ret);
    });
    imports.wbg.__wbindgen_is_null = function(arg0) {
        var ret = getObject(arg0) === null;
        return ret;
    };
    imports.wbg.__wbg_instanceof_IdbCursorWithValue_5057f6c875ee02f2 = function(arg0) {
        var ret = getObject(arg0) instanceof IDBCursorWithValue;
        return ret;
    };
    imports.wbg.__wbg_value_ff98e84154a96fed = handleError(function(arg0) {
        var ret = getObject(arg0).value;
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_delete_19145adb7bba22c1 = handleError(function(arg0) {
        var ret = getObject(arg0).delete();
        return addHeapObject(ret);
    });
    imports.wbg.__wbg_continue_84c9c467e80b692e = handleError(function(arg0) {
        getObject(arg0).continue();
    });
    imports.wbg.__wbg_oldVersion_06e2af273675b424 = function(arg0) {
        var ret = getObject(arg0).oldVersion;
        return ret;
    };
    imports.wbg.__wbg_newVersion_8ba775db0ed0c577 = function(arg0, arg1) {
        var ret = getObject(arg1).newVersion;
        getFloat64Memory0()[arg0 / 8 + 1] = isLikeNone(ret) ? 0 : ret;
        getInt32Memory0()[arg0 / 4 + 0] = !isLikeNone(ret);
    };
    imports.wbg.__wbg_instanceof_IdbDatabase_a7d9c90ade7e8d96 = function(arg0) {
        var ret = getObject(arg0) instanceof IDBDatabase;
        return ret;
    };
    imports.wbg.__wbg_transaction_4b85cfc358177e35 = function(arg0) {
        var ret = getObject(arg0).transaction;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_error_97e71550499436f4 = handleError(function(arg0) {
        var ret = getObject(arg0).error;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    });
    imports.wbg.__wbg_new_1d56e97b8de3067f = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var ret = new Error(v0);
        return addHeapObject(ret);
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
imports.wbg.__wbg_self_c0d3a5923e013647 = handleError(function() {
    var ret = self.self;
    return addHeapObject(ret);
});
imports.wbg.__wbg_window_7ee6c8be3432927d = handleError(function() {
    var ret = window.window;
    return addHeapObject(ret);
});
imports.wbg.__wbg_globalThis_c6de1d938e089cf0 = handleError(function() {
    var ret = globalThis.globalThis;
    return addHeapObject(ret);
});
imports.wbg.__wbg_global_c9a01ce4680907f8 = handleError(function() {
    var ret = global.global;
    return addHeapObject(ret);
});
imports.wbg.__wbg_newnoargs_8aad4a6554f38345 = function(arg0, arg1) {
    var v0 = getCachedStringFromWasm0(arg0, arg1);
    var ret = new Function(v0);
    return addHeapObject(ret);
};
imports.wbg.__wbg_call_1f85aaa5836dfb23 = handleError(function(arg0, arg1) {
    var ret = getObject(arg0).call(getObject(arg1));
    return addHeapObject(ret);
});
imports.wbg.__wbg_instanceof_Window_17fdb5cd280d476d = function(arg0) {
    var ret = getObject(arg0) instanceof Window;
    return ret;
};
imports.wbg.__wbg_length_0f0e68fde7e14c19 = function(arg0) {
    var ret = getObject(arg0).length;
    return ret;
};
imports.wbg.__wbg_get_5fd9dd78e47d6ed2 = function(arg0, arg1) {
    var ret = getObject(arg0)[arg1 >>> 0];
    return addHeapObject(ret);
};
imports.wbg.__wbg_error_f372691b5e416ece = function(arg0) {
    var ret = getObject(arg0).error;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__wbg_setoncomplete_5c092274f39a069b = function(arg0, arg1) {
    getObject(arg0).oncomplete = getObject(arg1);
};
imports.wbg.__wbg_setonerror_14a99a111ce325bf = function(arg0, arg1) {
    getObject(arg0).onerror = getObject(arg1);
};
imports.wbg.__wbg_setonabort_75e8a9e14c9f709d = function(arg0, arg1) {
    getObject(arg0).onabort = getObject(arg1);
};
imports.wbg.__wbg_transaction_f768a30ef74bc77f = handleError(function(arg0, arg1, arg2) {
    var ret = getObject(arg0).transaction(getObject(arg1), takeObject(arg2));
    return addHeapObject(ret);
});
imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
    const obj = getObject(arg1);
    var ret = typeof(obj) === 'string' ? obj : undefined;
    var ptr0 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};
imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
    var ret = debugString(getObject(arg1));
    var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    getInt32Memory0()[arg0 / 4 + 1] = len0;
    getInt32Memory0()[arg0 / 4 + 0] = ptr0;
};
imports.wbg.__wbindgen_throw = function(arg0, arg1) {
    throw new Error(getStringFromWasm0(arg0, arg1));
};
imports.wbg.__wbindgen_rethrow = function(arg0) {
    throw takeObject(arg0);
};
imports.wbg.__wbg_then_8c23dce80c84c8fb = function(arg0, arg1) {
    var ret = getObject(arg0).then(getObject(arg1));
    return addHeapObject(ret);
};
imports.wbg.__wbg_then_300153bb889a5b4b = function(arg0, arg1, arg2) {
    var ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
    return addHeapObject(ret);
};
imports.wbg.__wbg_resolve_708df7651c8929b8 = function(arg0) {
    var ret = Promise.resolve(getObject(arg0));
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper207 = function(arg0, arg1, arg2) {
    var ret = makeMutClosure(arg0, arg1, 27, __wbg_adapter_29);
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper178 = function(arg0, arg1, arg2) {
    var ret = makeMutClosure(arg0, arg1, 27, __wbg_adapter_32);
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper336 = function(arg0, arg1, arg2) {
    var ret = makeMutClosure(arg0, arg1, 27, __wbg_adapter_26);
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper337 = function(arg0, arg1, arg2) {
    var ret = makeMutClosure(arg0, arg1, 27, __wbg_adapter_35);
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper206 = function(arg0, arg1, arg2) {
    var ret = makeMutClosure(arg0, arg1, 27, __wbg_adapter_41);
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper208 = function(arg0, arg1, arg2) {
    var ret = makeMutClosure(arg0, arg1, 27, __wbg_adapter_38);
    return addHeapObject(ret);
};

if (typeof input === 'string' || (typeof Request === 'function' && input instanceof Request) || (typeof URL === 'function' && input instanceof URL)) {
    input = fetch(input);
}

const { instance, module } = await load(await input, imports);

wasm = instance.exports;
init.__wbindgen_wasm_module = module;
wasm.__wbindgen_start();
return wasm;
}

init(chrome.runtime.getURL("js/background.wasm")).catch(console.error);
//# sourceMappingURL=background.js.map
