// taken from: https://github.com/y21/dash/blob/master/crates/dash_rt/js/inspect.js
// consider changing it upstream too if you make a change here

const is = {
    string: (value) => typeof value === 'string',
    number: (value) => typeof value === 'number',
    boolean: (value) => typeof value === 'boolean',
    nullish: (value) => value === null || value === undefined,
    // error: (value) => value instanceof Error ,
    // ^ do this once it works again upstream
    // right now the suberror types don't inherit from Error
    error: (value) => [
        Error,
        TypeError,
        RangeError,
        SyntaxError,
        ReferenceError
    ].some(x => value instanceof x),
    array: (value) => value instanceof Array, // TODO: Array.isArray
    function: (value) => typeof value === 'function',
    looseObject: function (value) {
        return !this.nullish(value) && typeof value === 'object';
    },
    strictObject: function (value) {
        // TODO: use Array.isArray once we have it
        return this.looseObject(value) && !(value instanceof Array);
    }
};

function inner(value, depth) {
    if (is.string(value)) {
        if (depth > 0) {
            return '"' + value + '"';
        } else {
            return value;
        }
    }

    if (is.error(value)) {
        return value.stack;
    }

    if (is.strictObject(value)) {
        const keys = Object.keys(value);
        const hasElements = keys.length > 0;

        let repr;
        if (value.constructor !== Object) {
            repr = value.constructor.name + ' {';
        } else {
            repr = '{';
        }

        if (hasElements) repr += ' ';

        for (let i = 0; i < keys.length; i++) {
            if (i > 0) {
                repr += ', ';
            }

            const key = keys[i];
            repr += key + ': ' + inner(value[key], depth + 1);
        }

        if (hasElements) repr += ' ';

        repr += '}';

        return repr;
    }

    if (is.array(value)) {
        const len = value.length;

        let repr = '[';

        for (let i = 0; i < len; i++) {
            if (i > 0) {
                repr += ', ';
            }

            repr += inner(value[i], depth + 1);
        }

        repr += ']';
        return repr;
    }

    if (is.function(value)) {
        const name = value.name || '(anonymous)';

        return '[Function: ' + name + ']';
    }

    // if nothing matched, stringify
    return String(value);
}

(function (value) {
    return inner(value, 0);
})
