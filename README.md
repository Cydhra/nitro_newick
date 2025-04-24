# Minimal Newick Parser
A minimal newick parser that constructs a simple adjacency-list-based tree structure, or can construct arbitrary tree types via a builder trait.
The built-in tree structure is lightweight (no overhead that is not defined in Newick), implements `Send` and `Sync`, and supports modification.
The parser is a fast LL(1) single-pass parser without recursion to avoid memory problems during parsing.
