# mirp
**M**inecraft **I**nternal **R**edirection **P**roxy

## Getting Started
All you need is the Rust toolchain!
- Run `cargo run` to be walked through setup
- Enter your server info in the created `.env` file
- Rerun the binary to start listening

## Configuring a domain
If you are looking to register a domain, you can buy some for a small amount of money per year (typically about $10). There are many resources on how to set this up, and some detail setting it up for the purposes of a minecraft server. This is left as an exercise to the reader.

## What is this?
Mirp listens on a specified port which you must have configured to an external address. It is tested and used with `cloudflared` for a personal server, but other providers may work. Since these domains don't support Minecraft by default, you still have to expose a port on your local network. Mirp sits in between your minecraft server and exposed port, forwarding good packets and blocking naughty behavior. This lets you run minecraft entirely locally with the exposed port not having any real external affiliation with your server.

While this is more secure than raw port forwarding your server, it is not perfect. If you're looking for something more robust, you should check out external hosting providers like [playit.gg](playit.gg) or more robust tools like [Infrared](https://github.com/haveachin/infrared). You can also use more expensive options like Cloudflares pro plan.

You can also use mods or enforce your server members to run a local instance of `cloudflared`  which allows you to drop this 'middle-man' approach altogether.

## Disclaimer
Obviously, this software is provided without warranty. If you don't trust it, don't use it.
