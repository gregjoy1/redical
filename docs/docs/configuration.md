## RediCal configuration

The following documentation describes all available RediCal specific configuration accessible via the Redis `CONFIG SET ...` and `CONFIG GET ...` commands.

### `REDICAL.ICAL-PARSER-TIMEOUT-MS`

This configuration determines the timeout budget (in milliseconds) allocated to iCal parsing in the following commands:
* [RDCL.EVT_SET](../commands/rdcl.evt_set.md)
* [RDCL.EVO_SET](../commands/rdcl.evo_set.md)
* [RDCL.EVI_QUERY](../commands/rdcl.evi_query.md)

This is necessary in the control and prevention of computationally expensive iCal payloads (malicious or unintentional) that result in the process hanging for an excessive length of time contributing to overall disrupting.

Any command where the parsing of provided iCal exceeds the specified timeout will return early with an error. 

Currently the default value is 500ms, but can be set to any value from 1ms all the way to 60000ms (60s). 

#### Examples

Get the current configured `REDICAL.ICAL-PARSER-TIMEOUT-MS` value:
```bash
redis> CONFIG GET REDICAL.ICAL-PARSER-TIMEOUT-MS
1) "REDICAL.ICAL-PARSER-TIMEOUT-MS"
2) "500"
```

Set the configured `REDICAL.ICAL-PARSER-TIMEOUT-MS` value to 10 seconds:
```bash
redis> CONFIG SET REDICAL.ICAL-PARSER-TIMEOUT-MS 10000
OK
```

Set the configured `REDICAL.ICAL-PARSER-TIMEOUT-MS` value to 10 seconds in the `redis.conf` file:
```bash
...

REDICAL.ICAL-PARSER-TIMEOUT-MS 1
```
