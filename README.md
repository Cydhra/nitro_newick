# Flexible Newick Parser
A minimal newick parser designed for maximum flexibility.
By default, it constructs a simple adjacency-list-based tree structure.
This built-in tree structure is lightweight (no overhead that is not required for Newick), implements `Send` and `Sync`, and fully supports modification.
But the parser can also be used to construct arbitrary types using a simple builder trait.
The parser is a fast LL(1) single-pass parser without recursion for speed and lower memory footprint.

A serializer is provided to convert trees back into Newick format. 
The same philosophy of flexibility is applied here, allowing downstream crates to control certain ambiguities in the Newick format.

# Usage
In the simplest case, construct a `Parser` with a `SimpleTreeBuilder` and call `parse()` until `Ok(None)` is returned,
to parse all trees in the input to `NTree` instances.

```rs
let newick = "(A, B, (D, E):0.2)The_Root;";
let mut parser = Parser::new(newick.as_bytes(), SimpleTreeBuilder::new());
let tree: Result<Option<NTree>, ParseError> = parser.parse();
```

Multiple defaults can be changed with a `Settings` instance.
For example, the translation of underscores in labels to spaces can be disabled:
```rs
let mut parser = Parser::with_settings(
    newick.as_bytes(),
    SimpleTreeBuilder::new(),
    Settings::default().translate_underscores(false)
);
```

The `SimpleTreeBuilder` constructs `NTree` instances which is a simple tree structure based on doubly-linked Nodes.
It does not calculate or store extraneous information, and is designed to be a lightweight and flexible tree structure.

An optional optimization can be enabled with the crate feature `smallvec`, which optimizes the tree structure for binary trees using the smallvec crate.

If you want to parse Newick into your own tree structure, simply implement the `TreeBuilder` trait and give the parser an instance of your implementation instead of a `SimpleTreeBuilder`. 
The analogue `TreeSerialize` trait enables serialization using the built-in `Serializer`.

# Manipulation
The `NTree` type which serves as a minimal tree structure can be manipulated with all common tree operations.
Here are some examples for various operations.
Refer to the documentation to find the full API.

```rs
// Traversal:
let order = tree.postorder(tree.virtual_root().unwrap());
let order = tree.preorder(tree.virtual_root().unwrap());
let ordered_nodes = tree.traverse(order).collect::<Vec<_>>();

// Rooting:
tree.reroot(new_root_id);

// Node manipulation
// add a new node with a label and no support value, and an expected number of edges
let new_node_id = tree.add_node(Some("new node"), 1);

// connect it to the root with a branch length of 0.44 and no support value:
tree.add_edge(tree.virtual_root()?, new_node_id, None, Some(0.44));

// add a support value to the edge at a later point:
tree.update_edge(tree.virtual_root()?, new_node_id, Some(90.0), Some(0.44));
```

If a node has a label and a support value at the same time, the serializer has to prefer one of them.
This behavior can be configured.

# Serialization
Serialization works with an analogous type to parsing, and takes the same settings object.
For example, this is how to serialize a tree while preferring labels to support values for nodes that have both.
```rs
let serializer = Serializer::with_settings(Settings::default().prefer_labels(true));
let converted = serializer.serialize(&tree);
```