# Graphmatch

A low-level utility for matching a string against a directed acyclic graph of regexes.

- It supports matching strings partially too.
- It supports fine-grained control over which nodes in the graph can be matched partially or not.
- It supports compiling the whole graph to a regex, even for partial matches.
- The graph will always be matched against the input string from the very start.
- RegExp flags are supported as long as they are the same for all the regexes in the graph.

## Install

```sh
npm install graphmatch
```

## Usage

```ts
import graphmatch from 'graphmatch';

// Let's say we would like to match against this glob: foo/{bar,baz}/qux
// Let's express that as a graph of regexes that this library can match against
// Whether you reuse the "qux" node or not doesn't matter, both are supported

const GRAPH = {
  regex: /foo/,
  children: [
    {
      regex: /\//,
      children: [
        {
          regex: /bar/,
          children: [
            {
              regex: /\//,
              children: [
                {
                  regex: /qux/
                }
              ]
            }
          ]
        },
        {
          regex: /baz/,
          children: [
            {
              regex: /\//,
              children: [
                {
                  regex: /qux/
                }
              ]
            }
          ]
        }
      ]
    }
  ]
};

// Let's now match against the graph, fully

graphmatch ( GRAPH, 'foo/bar/qux' ); // => true
graphmatch ( GRAPH, 'foo/baz/qux' ); // => true

graphmatch ( GRAPH, 'foo/bar/whoops' ); // => false
graphmatch ( GRAPH, 'foo/baz' ); // => false

// Let's now match against the graph, partially
// A partial match happens when any node in the graph sees the end of the string, either before or after the node's regex is executed

graphmatch ( GRAPH, 'foo/bar/qux', { partial: true } ); // => true
graphmatch ( GRAPH, 'foo/bar/', { partial: true } ); // => true
graphmatch ( GRAPH, 'foo/bar', { partial: true } ); // => true
graphmatch ( GRAPH, 'foo/', { partial: true } ); // => true
graphmatch ( GRAPH, 'foo', { partial: true } ); // => true

graphmatch ( GRAPH, 'foo/bar/whoops', { partial: true } ); // => false
graphmatch ( GRAPH, 'foo/barsomething', { partial: true } ); // => false
graphmatch ( GRAPH, 'bar', { partial: true } ); // => false

// As we just saw partial matching is set to "false" by default, and there is an option to set it to "true" by default
// But you can also decide on a node-by-node basis whether partial matching should be enabled or disabled for that particular node
// A partial node will match if the whole input string has been consumed, right before or after the node's regex is executed
// This has higher priority compared to the global setting
// This is useful for fine-grained control over which nodes can be matched partially

const NODE_NEVER_PARTIAL = { // This node will never match partially
  partial: false,
  regex: /foo/,
  children: [],
};

const NODE_ALWAYS_PARTIAL = { // This node will always match partially
  partial: true,
  regex: /foo/,
  children: [],
};

// Let's now compile the whole graph to a single regex
// This is useful if you expect to match against the graph multiple times
// It's faster to compile the graph once and match against it multiple times

const fullRe = graphmatch.compile ( GRAPH ); // => RegExp

fullRe.test ( 'foo/bar/qux' ); // => true
fullRe.test ( 'foo/bar' ); // => false

const partialRe = graphmatch.compile ( GRAPH, { partial: true } ); // => RegExp

partialRe.test ( 'foo/bar/qux' ); // => true
partialRe.test ( 'foo/bar' ); // => true
```

## License

MIT © Fabio Spampinato
