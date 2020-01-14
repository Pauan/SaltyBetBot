function set_property(obj, name, value) { obj[name] = value; }

    function add_event(elem, name, f) {
        elem.addEventListener(name, f, {
            capture: false,
            once: false,
            passive: true
        });
    }

    function remove_event(elem, name, f) {
        elem.removeEventListener(name, f, false);
    }

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

    // TODO add to js_sys
    function format_float(f) {
        return f.toLocaleString("en-US", {
            style: "currency",
            currency: "USD",
            minimumFractionDigits: 0
        });
    }

    // TODO add to js_sys
    function decimal(f) {
        return f.toLocaleString("en-US", {
            style: "decimal",
            maximumFractionDigits: 2
        });
    }

    function set_utc_date(date, days) {
        date.setUTCDate(days);
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
function __wbg_adapter_26(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures__invoke0_mut__h37398151ea42af0d(arg0, arg1);
}

function __wbg_adapter_29(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures__invoke1_mut__hbf543a7afe63658a(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_32(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures__invoke0_mut__h37398151ea42af0d(arg0, arg1);
}

function __wbg_adapter_35(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures__invoke1_mut__hbf543a7afe63658a(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_38(arg0, arg1, arg2) {
    var ptr0 = passStringToWasm0(arg2, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    var len0 = WASM_VECTOR_LEN;
    wasm.wasm_bindgen__convert__closures__invoke1_mut__hd2f10f59e8756e7e(arg0, arg1, ptr0, len0);
}

let stack_pointer = 32;

function addBorrowedObject(obj) {
    if (stack_pointer == 1) throw new Error('out of js stack');
    heap[--stack_pointer] = obj;
    return stack_pointer;
}
function __wbg_adapter_41(arg0, arg1, arg2) {
    try {
        wasm.wasm_bindgen__convert__closures__invoke1_mut_ref__hdeb6ff529dc032af(arg0, arg1, addBorrowedObject(arg2));
    } finally {
        heap[stack_pointer++] = undefined;
    }
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

function getArrayU8FromWasm0(ptr, len) {
    return getUint8Memory0().subarray(ptr / 1, ptr / 1 + len);
}

function init(module) {
    if (typeof module === 'undefined') {
        module = import.meta.url.replace(/\.js$/, '_bg.wasm');
    }
    let result;
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbindgen_cb_forget = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        var ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__widl_f_checked_HTMLInputElement = function(arg0) {
        var ret = getObject(arg0).checked;
        return ret;
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
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
    imports.wbg.__wbg_removeevent_73b2322cc4310f4b = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        remove_event(getObject(arg0), v0, getObject(arg3));
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        var ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__widl_f_error_1_ = function(arg0) {
        console.error(getObject(arg0));
    };
    imports.wbg.__wbg_formatfloat_805fb4692319aae4 = function(arg0, arg1) {
        var ret = format_float(arg1);
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__widl_f_log_1_ = function(arg0) {
        console.log(getObject(arg0));
    };
    imports.wbg.__widl_f_class_list_Element = function(arg0) {
        var ret = getObject(arg0).classList;
        return addHeapObject(ret);
    };
    imports.wbg.__widl_f_add_1_DOMTokenList = function(arg0, arg1, arg2) {
        try {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            getObject(arg0).add(v0);
        } catch (e) {
            handleError(e);
        }
    };
    imports.wbg.__widl_f_value_HTMLSelectElement = function(arg0, arg1) {
        var ret = getObject(arg1).value;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__widl_f_value_HTMLInputElement = function(arg0, arg1) {
        var ret = getObject(arg1).value;
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__widl_f_client_x_MouseEvent = function(arg0) {
        var ret = getObject(arg0).clientX;
        return ret;
    };
    imports.wbg.__widl_f_client_width_Element = function(arg0) {
        var ret = getObject(arg0).clientWidth;
        return ret;
    };
    imports.wbg.__widl_f_set_timeout_with_callback_and_timeout_and_arguments_0_Window = function(arg0, arg1, arg2) {
        try {
            var ret = getObject(arg0).setTimeout(getObject(arg1), arg2);
            return ret;
        } catch (e) {
            handleError(e);
        }
    };
    imports.wbg.__wbg_now_65115a9aef2f5270 = function() {
        var ret = Date.now();
        return ret;
    };
    imports.wbg.__wbg_set_c3a2ba27703a6186 = function(arg0, arg1, arg2) {
        try {
            var ret = Reflect.set(getObject(arg0), getObject(arg1), getObject(arg2));
            return ret;
        } catch (e) {
            handleError(e);
        }
    };
    imports.wbg.__widl_instanceof_HTMLElement = function(arg0) {
        var ret = getObject(arg0) instanceof HTMLElement;
        return ret;
    };
    imports.wbg.__wbg_addevent_5d5369f0d7dd3732 = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        add_event(getObject(arg0), v0, getObject(arg3));
    };
    imports.wbg.__widl_f_style_HTMLElement = function(arg0) {
        var ret = getObject(arg0).style;
        return addHeapObject(ret);
    };
    imports.wbg.__widl_instanceof_SVGElement = function(arg0) {
        var ret = getObject(arg0) instanceof SVGElement;
        return ret;
    };
    imports.wbg.__widl_f_set_attribute_Element = function(arg0, arg1, arg2, arg3, arg4) {
        try {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            var v1 = getCachedStringFromWasm0(arg3, arg4);
            getObject(arg0).setAttribute(v0, v1);
        } catch (e) {
            handleError(e);
        }
    };
    imports.wbg.__widl_instanceof_HTMLSelectElement = function(arg0) {
        var ret = getObject(arg0) instanceof HTMLSelectElement;
        return ret;
    };
    imports.wbg.__widl_instanceof_HTMLInputElement = function(arg0) {
        var ret = getObject(arg0) instanceof HTMLInputElement;
        return ret;
    };
    imports.wbg.__widl_f_set_text_content_Node = function(arg0, arg1, arg2) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        getObject(arg0).textContent = v0;
    };
    imports.wbg.__wbg_setproperty_d6ae5d3843d05915 = function(arg0, arg1, arg2, arg3) {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        set_property(getObject(arg0), v0, getObject(arg3));
    };
    imports.wbg.__widl_f_remove_1_DOMTokenList = function(arg0, arg1, arg2) {
        try {
            var v0 = getCachedStringFromWasm0(arg1, arg2);
            getObject(arg0).remove(v0);
        } catch (e) {
            handleError(e);
        }
    };
    imports.wbg.__wbg_decimal_38874f68d559be18 = function(arg0, arg1) {
        var ret = decimal(arg1);
        var ptr0 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len0 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len0;
        getInt32Memory0()[arg0 / 4 + 0] = ptr0;
    };
    imports.wbg.__widl_f_append_child_Node = function(arg0, arg1) {
        try {
            var ret = getObject(arg0).appendChild(getObject(arg1));
            return addHeapObject(ret);
        } catch (e) {
            handleError(e);
        }
    };
    imports.wbg.__widl_f_insert_before_Node = function(arg0, arg1, arg2) {
        try {
            var ret = getObject(arg0).insertBefore(getObject(arg1), getObject(arg2));
            return addHeapObject(ret);
        } catch (e) {
            handleError(e);
        }
    };
    imports.wbg.__widl_f_replace_child_Node = function(arg0, arg1, arg2) {
        try {
            var ret = getObject(arg0).replaceChild(getObject(arg1), getObject(arg2));
            return addHeapObject(ret);
        } catch (e) {
            handleError(e);
        }
    };
    imports.wbg.__widl_f_remove_child_Node = function(arg0, arg1) {
        try {
            var ret = getObject(arg0).removeChild(getObject(arg1));
            return addHeapObject(ret);
        } catch (e) {
            handleError(e);
        }
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
    imports.wbg.__wbg_sendmessageraw_06db037e50adb455 = function(arg0, arg1) {
        var v0 = getCachedStringFromWasm0(arg0, arg1);
        var ret = send_message_raw(v0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_b43fc449db38d3bd = function(arg0, arg1) {
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
imports.wbg.__widl_f_document_Window = function(arg0) {
    var ret = getObject(arg0).document;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__widl_f_create_element_Document = function(arg0, arg1, arg2) {
    try {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var ret = getObject(arg0).createElement(v0);
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_create_element_ns_Document = function(arg0, arg1, arg2, arg3, arg4) {
    try {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var v1 = getCachedStringFromWasm0(arg3, arg4);
        var ret = getObject(arg0).createElementNS(v0, v1);
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_create_text_node_Document = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    var ret = getObject(arg0).createTextNode(v0);
    return addHeapObject(ret);
};
imports.wbg.__widl_f_remove_property_CSSStyleDeclaration = function(arg0, arg1, arg2, arg3) {
    try {
        var v0 = getCachedStringFromWasm0(arg2, arg3);
        var ret = getObject(arg1).removeProperty(v0);
        var ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_body_Document = function(arg0) {
    var ret = getObject(arg0).body;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__widl_f_set_property_with_priority_CSSStyleDeclaration = function(arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
    try {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var v1 = getCachedStringFromWasm0(arg3, arg4);
        var v2 = getCachedStringFromWasm0(arg5, arg6);
        getObject(arg0).setProperty(v0, v1, v2);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_get_property_value_CSSStyleDeclaration = function(arg0, arg1, arg2, arg3) {
    try {
        var v0 = getCachedStringFromWasm0(arg2, arg3);
        var ret = getObject(arg1).getPropertyValue(v0);
        var ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        var len1 = WASM_VECTOR_LEN;
        getInt32Memory0()[arg0 / 4 + 1] = len1;
        getInt32Memory0()[arg0 / 4 + 0] = ptr1;
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_css_rules_CSSStyleSheet = function(arg0) {
    try {
        var ret = getObject(arg0).cssRules;
        return addHeapObject(ret);
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_length_CSSRuleList = function(arg0) {
    var ret = getObject(arg0).length;
    return ret;
};
imports.wbg.__widl_f_insert_rule_with_index_CSSStyleSheet = function(arg0, arg1, arg2, arg3) {
    try {
        var v0 = getCachedStringFromWasm0(arg1, arg2);
        var ret = getObject(arg0).insertRule(v0, arg3 >>> 0);
        return ret;
    } catch (e) {
        handleError(e);
    }
};
imports.wbg.__widl_f_get_CSSRuleList = function(arg0, arg1) {
    var ret = getObject(arg0)[arg1 >>> 0];
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__widl_f_style_CSSStyleRule = function(arg0) {
    var ret = getObject(arg0).style;
    return addHeapObject(ret);
};
imports.wbg.__widl_f_set_type_HTMLStyleElement = function(arg0, arg1, arg2) {
    var v0 = getCachedStringFromWasm0(arg1, arg2);
    getObject(arg0).type = v0;
};
imports.wbg.__widl_f_head_Document = function(arg0) {
    var ret = getObject(arg0).head;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__widl_f_sheet_HTMLStyleElement = function(arg0) {
    var ret = getObject(arg0).sheet;
    return isLikeNone(ret) ? 0 : addHeapObject(ret);
};
imports.wbg.__widl_f_clear_timeout_with_handle_Window = function(arg0, arg1) {
    getObject(arg0).clearTimeout(arg1);
};
imports.wbg.__wbg_new_3a746f2619705add = function(arg0, arg1) {
    var v0 = getCachedStringFromWasm0(arg0, arg1);
    var ret = new Function(v0);
    return addHeapObject(ret);
};
imports.wbg.__wbg_call_f54d3a6dadb199ca = function(arg0, arg1) {
    var ret = getObject(arg0).call(getObject(arg1));
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_jsval_eq = function(arg0, arg1) {
    var ret = getObject(arg0) === getObject(arg1);
    return ret;
};
imports.wbg.__wbg_self_ac379e780a0d8b94 = function(arg0) {
    var ret = getObject(arg0).self;
    return addHeapObject(ret);
};
imports.wbg.__wbg_crypto_1e4302b85d4f64a2 = function(arg0) {
    var ret = getObject(arg0).crypto;
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_is_undefined = function(arg0) {
    var ret = getObject(arg0) === undefined;
    return ret;
};
imports.wbg.__wbg_getRandomValues_1b4ba144162a5c9e = function(arg0) {
    var ret = getObject(arg0).getRandomValues;
    return addHeapObject(ret);
};
imports.wbg.__wbg_require_6461b1e9a0d7c34a = function(arg0, arg1) {
    var v0 = getCachedStringFromWasm0(arg0, arg1);
    var ret = require(v0);
    return addHeapObject(ret);
};
imports.wbg.__wbg_randomFillSync_1b52c8482374c55b = function(arg0, arg1, arg2) {
    getObject(arg0).randomFillSync(getArrayU8FromWasm0(arg1, arg2));
};
imports.wbg.__wbg_getRandomValues_1ef11e888e5228e9 = function(arg0, arg1, arg2) {
    getObject(arg0).getRandomValues(getArrayU8FromWasm0(arg1, arg2));
};
imports.wbg.__wbindgen_number_new = function(arg0) {
    var ret = arg0;
    return addHeapObject(ret);
};
imports.wbg.__wbg_new_1a9b94a86cbfa090 = function(arg0) {
    var ret = new Date(getObject(arg0));
    return addHeapObject(ret);
};
imports.wbg.__wbg_setUTCMinutes_175224dd9ffe7c33 = function(arg0, arg1) {
    var ret = getObject(arg0).setUTCMinutes(arg1 >>> 0);
    return ret;
};
imports.wbg.__wbg_setUTCSeconds_b97ef930ffca00fb = function(arg0, arg1) {
    var ret = getObject(arg0).setUTCSeconds(arg1 >>> 0);
    return ret;
};
imports.wbg.__wbg_setUTCMilliseconds_f3e3a07835b392ed = function(arg0, arg1) {
    var ret = getObject(arg0).setUTCMilliseconds(arg1 >>> 0);
    return ret;
};
imports.wbg.__wbg_getTime_f2424a06ce25bf74 = function(arg0) {
    var ret = getObject(arg0).getTime();
    return ret;
};
imports.wbg.__wbg_getUTCDate_7abc6db2c28b1ad0 = function(arg0) {
    var ret = getObject(arg0).getUTCDate();
    return ret;
};
imports.wbg.__wbg_setutcdate_0f94150d55159892 = function(arg0, arg1) {
    set_utc_date(getObject(arg0), arg1);
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
imports.wbg.__wbg_then_79de0b6809569306 = function(arg0, arg1) {
    var ret = getObject(arg0).then(getObject(arg1));
    return addHeapObject(ret);
};
imports.wbg.__wbg_resolve_4e9c46f7e8321315 = function(arg0) {
    var ret = Promise.resolve(getObject(arg0));
    return addHeapObject(ret);
};
imports.wbg.__wbg_then_5a9068d7b674caf9 = function(arg0, arg1, arg2) {
    var ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
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
imports.wbg.__widl_instanceof_Window = function(arg0) {
    var ret = getObject(arg0) instanceof Window;
    return ret;
};
imports.wbg.__wbindgen_closure_wrapper258 = function(arg0, arg1, arg2) {

    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (arg0) => {
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return __wbg_adapter_38(a, state.b, arg0);
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(25)(a, state.b);
            else state.a = a;
        }
    }
    ;
    real.original = state;
    var ret = real;
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper892 = function(arg0, arg1, arg2) {

    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (arg0) => {
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return __wbg_adapter_29(a, state.b, arg0);
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(25)(a, state.b);
            else state.a = a;
        }
    }
    ;
    real.original = state;
    var ret = real;
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper220 = function(arg0, arg1, arg2) {

    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = () => {
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return __wbg_adapter_32(a, state.b, );
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(25)(a, state.b);
            else state.a = a;
        }
    }
    ;
    real.original = state;
    var ret = real;
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper261 = function(arg0, arg1, arg2) {

    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (arg0) => {
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return __wbg_adapter_41(a, state.b, arg0);
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(25)(a, state.b);
            else state.a = a;
        }
    }
    ;
    real.original = state;
    var ret = real;
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper625 = function(arg0, arg1, arg2) {

    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = () => {
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return __wbg_adapter_26(a, state.b, );
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(25)(a, state.b);
            else state.a = a;
        }
    }
    ;
    real.original = state;
    var ret = real;
    return addHeapObject(ret);
};
imports.wbg.__wbindgen_closure_wrapper877 = function(arg0, arg1, arg2) {

    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (arg0) => {
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return __wbg_adapter_35(a, state.b, arg0);
        } finally {
            if (--state.cnt === 0) wasm.__wbindgen_export_2.get(25)(a, state.b);
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

init(chrome.runtime.getURL("js/chart.wasm")).catch(console.error);
//# sourceMappingURL=chart.js.map
