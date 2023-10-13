# huion-solar-getter
A _very_ basic server that logs Huion SUN2000 inverter data to a Redis instance.
## What?
The 2 main things this program does are:
- read registers from Huion SUN2000 solar inverters
- write the values to Redis Timeseries with corresponding keys
## How?
1. Enable Modbus-TCP in your inverter's settings
2. Compile the program yourself _(or download a release if I figure out github actions)_
3. Set `INV_IP` and `RD_IP` environment variables to the corresponding IP's in your network
4. Run the executable. The data should start appearing in your Redis instance every ~90 seconds
## Why?
The SUN2000 solar inverters from Huion only allow their data to be viewed over Huion's FusionSolar website, so getting to the raw numbers is basically impossible.
Of course this is annoying if you'd want to create a custom dashboard or do anything other than look at fancy graphs.
Somehow this project has turned into me trying to reverse-engineer/understand the 130 page Modbus Interface Definition (which is impossible to find online by the way). <br/>
A couple of **nice** features the inverter has:
- Updating some values very irregularly _or not at all_.
- Getting overwhelmed by requests easily.
- Scaling values in ways that only make sense to aliens.
- ...
## Todos
If you want to help this project, all contributions are welcome.
Some of the specific TODOs are:
- Expand the Register definition list in `definitions.json`
- Adapt the tool to work for inverter models other than SUN2000-8KTL-M1
- Improve my very hacky Rust code
