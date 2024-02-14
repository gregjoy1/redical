> [!NOTE]
> TODO

# <COMMAND>

### Syntax
```bash
<COMMAND> key value [value ...]
```

Description text.

## Required arguments

### key
the key to modify.

### value
one or more values to append

## Return value 

`<COMMAND>` returns an [array](https://redis.io/docs/reference/protocol-spec/#resp-arrays) of integer replies for each path, the array's new size, or `nil`, if the matching value is not an array. 
For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec). 

## Examples

Create a map with a sub-array.
```bash
# path: ["$"] 
# value: {"foo":["a","b","c"]}
redis> <COMMAND> key "\x81\x61$" "\xa1\x63foo\x83\x61a\x61b\x61c"
OK
```

Get the updated document.
```bash
# result: {"foo":["a","b","c","d"]}
redis> <COMMAND> key
"\x81\xa1cfoo\x84aaabacad"
```

## See also

[`COMMAND`](doc.path.md) | [`COMMAND`](doc.path.md)
