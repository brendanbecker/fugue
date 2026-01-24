# Why fugue Exists

Traditional terminal multiplexers treat output as opaque bytes.

This breaks down when:
- sessions last days instead of minutes
- automation runs alongside humans
- failure and recovery are normal
- AI agents participate in workflows

fugue exists to make terminal work:

- **Durable**  
  Sessions survive crashes and reconnects.

- **Observable**  
  State and intent are visible, not inferred.

- **Recoverable**  
  Failure is expected and convergent.

- **Shared**  
  Humans and agents operate in the same workspace.

- **Safe**  
  Automation never silently overrides human intent.

fugue does not replace tmux.
It occupies a different layer:

> tmux multiplexes terminals  
> fugue multiplexes **work**

The terminal is the delivery mechanism.
Coordination is the product.

