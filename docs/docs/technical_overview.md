# Technical overview

This document aims to provide an overview of the architecture of RediCal.

## Working data structure

* Redis
    * RediCal module
        * Calendar data type - This is the key data type the RediCal module provides, it contains events and calendar wide indexes of all contained events for querying.
            * UID - Mirrors the Redis key UUID.
            * Calendar wide indexes - All combined containing event indexes for querying all events defined within the calendar.
                * Categories
                * RelatedTo
                * Class
                * Geo
            * Events - Key/Value hashmap of event UID to Event data type.
                * Event - This represents an event defined within the calendar, it contains schedule information, indexed properties, passive properties, event wide indexes, and ocurrence overrides.
                    * UID
                    * ScheduleProperties
                        * RRule
                        * ExRule
                        * RDates
                        * ExDates
                        * Duration
                        * DTStart
                        * DTEnd
                    * IndexedProperties - Queryable indexed event level properties for querying.
                        * Categories
                        * RelatedTo
                        * Class
                        * Geo
                    * PassiveProperties - Non-indexed passive event level properties that are simply regurgitated on all extrapolated EventInstances.
                        * PassiveProperty
                    * EventOccurrenceOverrides - Key/Value hashmap of overriden event occurrence Unix timestamp to EventOccurrenceOverride data type.
                        * EventOccurrenceOverride - This represents an occurrence level override for the event it is contained within. This is baked into the calendar and event wide indexes for querying.
                            * DTStart
                            * DTEnd
                            * Duration
                        * IndexedProperties - Queryable indexed event occurrence override level properties for querying.
                            * Categories
                            * RelatedTo
                            * Class
                            * Geo
                        * PassiveProperties - Non-indexed passive event occurrence override level properties that are simply regurgitated on all extrapolated EventInstances.
                            * PassiveProperty
                    * Event wide indexes
                        * Categories
                        * RelatedTo
                        * Class
                        * Geo

## Workspaces

RediCal has three workspace members:
* `redical_ical`
* `redical_core`
* `redical_redis`

### `redical_ical`

This is primarily concerned with siloing the complexities of parsing and serializing the iCalendar content lines, properties, params, and values used extensively within RediCal.

### `redical_core`

This encapsulates the Calendar, Event, EventOccurrenceOverride data model itself, the indexes built and maintained across these entities, and all other core processes operating on the model.

Its worth noting that these act as containers for all the iCalendar properties defined in the `redical_ical` workspace.

### `redical_redis`

The outer layer of the "onion", bridging the gap between Redis and the `redical_core` processes.

This is the compilation entry point that generates the Redis module lib.

Also contained here is the definition and de/hydration of the persisted Calendar RDB data type.

Essentially a simplified intermediary representation of the core data model that is periodically persisted to disk by Redis. Designed with the intention to allow the core data model to change across different RediCal versions whilst maintaining compatibility with RDB dumps from earlier versions. Especially important whilst upgrading/migrating as it allows the user to dump the RediCal calendars to disk, restart the Redis server with another (updated) version of RediCal, and it work out of the box.
