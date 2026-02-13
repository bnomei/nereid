# Rendering Mermaid diagrams as ASCII

## 1 Mermaid diagram types

Mermaid diagrams are declared by a keyword at the beginning of the block and each type has its own syntax.  The official documentation lists many diagram types, including flowchart, sequence, class, state and entity‑relationship diagrams, user‑journey diagrams, Gantt charts, pie charts and more【754806014088699†L31-L82】.  For design purposes these diagrams can be grouped into categories that share similar layout properties:

| Category | Mermaid types (examples) | Layout characteristics |
| --- | --- | --- |
| **Flow‑based graphs** | flowcharts (`graph LR`, `graph TD`, etc.), Git graph, requirement diagrams, C4 diagrams | Directed or undirected node‑link diagrams; nodes can have different shapes and labels; edges may have arrow heads, labels or different styles; layout usually uses hierarchical or layered algorithms. |
| **Sequence diagrams** | `sequenceDiagram` | Participants drawn on a horizontal axis; messages shown as arrows between lifelines; time flows top‑down; support for activation boxes, self messages and loops. |
| **Hierarchical/class diagrams** | `classDiagram`, state diagrams (`stateDiagram‑v2`), mind‑maps, tree diagrams, block diagrams | Hierarchical relationships; nodes often nested; tree or radial layouts; may use tidy‑tree or layered layout. |
| **Tabular/relational** | Entity‑relationship (`erDiagram`), user‑journey, requirement diagrams | Entities linked by labelled relationships; labels may be cardinality or journeys; layout similar to flow‑based graphs but often denser. |
| **Charts** | Gantt, pie, quadrant, XY charts, radar, Sankey, timeline, treemap | Represent data rather than graphs; require plotting numeric values (bar lengths, slices, axes) rather than nodes. |

When targeting ASCII output, diagrams with a simple node‑link structure (flowcharts, class/state diagrams, ER diagrams and sequence diagrams) are the most practical.  Complex charts like Sankey or pie charts require text‑based plotting of numeric data and are better handled by specialised ASCII plotting libraries.

## 2 Complexity of rendering diagrams to ASCII

### 2.1 Graph layout problem

Mermaid diagrams rely on automatic layout algorithms to compute coordinates for nodes and edges.  Flowcharts and similar graphs typically use **layered graph drawing** (the Sugiyama framework).  This algorithm assigns each vertex of a directed graph to a horizontal layer, introduces dummy vertices for edges that cross layers, orders vertices in each layer to minimise edge crossings and computes coordinates【19896654984060†L117-L177】.  The algorithm solves several NP‑hard sub‑problems (feedback arc set, crossing minimisation), so practical implementations use heuristics【19896654984060†L117-L177】.  Mermaid uses different layout engines such as **ELK (Eclipse Layout Kernel)**, **Dagre** and **cose‑bilkent**【959796239811321†L141-L152】.  ELK provides a collection of layout algorithms and only computes positions; rendering is separate【728211225576261†L20-L30】.

In an ASCII renderer you need to replicate these layout steps:

- **Parse** the Mermaid code into an abstract syntax tree (AST) or intermediate graph representation.  The AST identifies nodes, edges, labels and diagram type.
- **Compute sizes**: determine the width/height of each node based on label length and padding.  ASCII uses monospaced characters, so the size equals character count + border thickness.
- **Layout**: apply a layout algorithm to assign nodes to layers and positions.  For flowcharts, the Sugiyama approach is appropriate.  Sequence diagrams use a simple horizontal alignment of participants and vertical placement of messages.  State and class diagrams can use tree layouts (e.g., tidy‑tree) or layered layouts.
- **Edge routing**: after positioning nodes, compute paths for edges—straight lines, poly‑lines or curves—using ASCII symbols.  Arrows can be represented by `-->`, `-.-`, etc.  Avoid overlapping edges/nodes by reserving space on the canvas.
- **Render**: draw nodes with borders using ASCII or Unicode box‑drawing characters, fill in labels, and draw connectors.  Provide options for pure ASCII (`+`/`-`/`|`) or Unicode (─│┌┐└┘).  Add padding between nodes horizontally (`paddingX`) and vertically (`paddingY`) to improve readability.

### 2.2 Features and limitations of ASCII

ASCII has constraints that affect the renderer:

- **Character set** – extended box‑drawing characters provide clean lines but may not be available in all terminals; pure ASCII uses `+`, `-`, `|`.  Libraries like **mermaid‑ascii** support a `--ascii` flag to force pure ASCII【665572273740704†L520-L527】.
- **Fixed‑width grid** – diagrams are drawn on a virtual grid where each cell holds a single character.  Node widths must be rounded to the nearest integer; long labels may wrap or require bigger boxes.
- **No colour or styling** – ASCII cannot replicate Mermaid’s themes; however, unicode shading and bold characters can emphasise edges or nodes.
- **Limited shapes** – only simple shapes (rectangles, diamonds, circles) can be approximated.  Complex shapes like cylinders or polygons become rectangles with labels.
- **Edge paths** – diagonal or curved edges are hard to render; most ASCII renderers use orthogonal routing: horizontal and vertical segments with corners.

### 2.3 Diagram‑specific considerations

- **Flowcharts & Git graphs** – require hierarchical layout; need to support different node shapes (rectangle, round corners, diamond for decisions).  ASCII representation approximates these as rectangles with text; diamond nodes can be drawn with slanted lines or replaced by “[ ]” with a label.  Parallel edges and bidirectional arrows must be drawn with spacing to avoid overlap.  mermaid‑ascii demonstrates this by adjusting horizontal/vertical spacing (`paddingX`, `paddingY`)【665572273740704†L360-L407】.
- **Sequence diagrams** – participants are columns; messages are horizontal arrows with optional labels.  The renderer must allocate equal column width per participant and draw vertical lifelines.  mermaid‑ascii shows support for solid (`->>`) and dotted (`-->>`) arrows and self‑messages【665572273740704†L560-L637】.  Activation boxes are tricky in ASCII; they may be drawn as vertical bars using `|` characters.
- **Class/ER diagrams** – these are hierarchical or relational graphs with compartments (e.g., class members).  Representing compartments in ASCII requires nested rectangles.  Layout can use a tree or layered algorithm with long horizontal lines connecting classes.  Support for inheritance and aggregation arrows would require arrowhead variations (e.g., `<|--`, `o--`).
- **State diagrams** – similar to flowcharts but include composite states.  ASCII can draw nested boxes; transitions can be drawn with labelled edges.  For simple state machines (no hierarchy), a flowchart renderer suffices.

## 3 Designing an ASCII Mermaid renderer

### 3.1 Parsing and AST

1. **Mermaid parser**: Write or reuse a parser that reads Mermaid code and produces an AST.  The AST should identify the diagram type, list of nodes/participants, edges with types (normal, dotted, thick), labels, node shapes, and subgraphs.  Libraries exist for JavaScript (used by Mermaid) but there is no mature Rust parser.  Options:
   * **Port an existing parser** or call the official Mermaid CLI via Node from Rust and parse its JSON output.
   * **Implement a minimal parser** that supports a subset of diagram types (e.g., flowcharts and sequence diagrams) with regular expressions and simple grammar rules.

2. **Graph representation**: Represent nodes and edges using a graph library such as `petgraph` in Rust.  Store node metadata (id, label, width, height, shape) and edge metadata (target, label, style).  For sequence diagrams, use a different structure: participants list and message list.

### 3.2 Layout computation

Use appropriate layout algorithms for each diagram type:

- **Layered graphs**: Implement a simplified Sugiyama layout: remove cycles by reversing edges, assign nodes to layers using longest‑path or Coffman–Graham heuristics, insert dummy nodes for multi‑layer edges, reorder nodes within layers to reduce crossings, and compute x/y positions【19896654984060†L117-L177】.  Existing Rust crates such as `petgraph` offer topological sorting, but crossing minimisation and edge routing may need custom heuristics.  Alternatively, call Graphviz’s `dot` or ELK via command line and parse the resulting positions.

- **Tree layout**: For hierarchical diagrams (mind maps, class hierarchies), apply a tidy tree algorithm that positions children evenly around parents.  Some algorithms have linear‑time complexity.

- **Sequence diagrams**: Assign equal horizontal spacing to participants and compute y‑coordinates for each message sequentially; handle loops or alternatives by reserving extra vertical space.

- **Manual overrides**: Provide configuration to adjust orientation (`LR`, `TD`, `RL`, `BT`) and spacing.  mermaid‑ascii exposes `-x` (horizontal) and `-y` (vertical) padding parameters to control node spacing【665572273740704†L360-L407】.

### 3.3 Rendering to ASCII

After computing positions, render to a 2‑D grid:

1. **Virtual canvas**: Create a 2‑D array of characters representing rows and columns.  The canvas size equals the max x and y coordinates multiplied by node width/height plus padding.

2. **Draw nodes**:
   * Choose either Unicode box‑drawing characters or pure ASCII.  For Unicode, use `─`, `│`, `┌`, `┐`, `└`, `┘` to draw borders.  For pure ASCII, use `-`, `|`, `+`【665572273740704†L520-L527】.
   * Write the label centred inside the box; pad with spaces to match the box width.  Provide a `boxBorderPadding` setting to control inner spacing【312636815307403†L169-L174】.

3. **Draw edges**:
   * For horizontal or vertical lines, fill with `─`/`│` (or `-`/`|`).  For corners, use `┌`/`┐`/`└`/`┘` (or `+`).
   * Arrowheads can be rendered using `►`/`◄` for Unicode or `>`/`<` for ASCII.
   * Dotted lines can be approximated by alternating characters such as `┈`/`·` or `.`【665572273740704†L560-L595】.
   * For sequence diagrams, draw lifelines (`│`) beneath participant boxes and messages as horizontal arrows with labels above them【665572273740704†L560-L637】.

4. **Label positioning**: For edge labels (e.g., decisions in flowcharts), compute the midpoint of the connecting segment and write the label above or below the line, leaving enough space.

5. **Spacing adjustment**: After initial placement, perform a **post‑layout pass** to resolve overlaps: shift nodes or edges and increase padding.  This may require iterative adjustments until no overlaps occur or until a maximum canvas size is reached.

### 3.4 Design considerations for a Rust implementation

- **Language features**: Rust’s strong type system and performance make it suitable for implementing parsers and layout algorithms.  Use crates like `nom` or `pest` for parsing, `petgraph` for graph representation and traversal, and `unicode‑width` for calculating character widths.
- **Third‑party layout engines**: Integrating with existing layout engines (Graphviz `dot` or ELK) via command‑line calls can offload complex layout tasks.  The renderer can then read node coordinates and convert them to ASCII positions.
- **Extensibility**: Design trait‑based interfaces for diagram types so that adding support for new diagrams requires implementing `parse`, `layout` and `render` traits.
- **Testing**: Create snapshot tests comparing rendered diagrams against expected ASCII output to ensure stability.

## 4 Existing ASCII rendering libraries and tools

Several open‑source projects already render Mermaid or other text diagrams to ASCII.  Studying them provides insight into design choices:

- **mermaid‑ascii** (Golang).  A command‑line program and web service that converts Mermaid code to ASCII.  The README demonstrates rendering flowchart diagrams and sequence diagrams; users can adjust horizontal and vertical spacing (`-x`, `-y`), border padding, and choose pure ASCII output【665572273740704†L360-L407】【665572273740704†L560-L689】.  Sequence diagrams support solid and dotted arrows and multiple participants【665572273740704†L560-L637】.  The project uses the official Mermaid parser and layout engine internally and then renders the results to ASCII.  It is open source (MIT license).  Although written in Go, its algorithms (parsing AST, computing sizes, orthogonal routing) can inspire a Rust implementation.

- **beautiful‑mermaid** (JavaScript).  A library by Craft that renders Mermaid diagrams as SVG or ASCII art.  It is “ultra‑fast, fully themeable, and outputs to both SVG and ASCII”【13940927009259†L100-L107】.  It supports flowchart, state, sequence, class and ER diagrams【312636815307403†L82-L90】.  The library exposes a `renderMermaidAscii()` function with options for Unicode/ASCII output, horizontal/vertical spacing and box padding【312636815307403†L156-L174】.  The documentation shows simple examples and lists built‑in themes.  The ASCII renderer is based on mermaid‑ascii【13940927009259†L100-L117】.

- **Graph‑Easy** (Perl).  A library that converts simple text notation into ASCII diagrams.  It is used for documenting software architecture and flowcharts.  The tool supports multiple output formats (ASCII art, Unicode box art, Graphviz DOT, HTML) and can embed diagrams in README files【272636485501062†L114-L127】.  Although it does not parse Mermaid syntax, its ASCII rendering engine demonstrates how to draw boxes, arrows and labels on a character grid.

- **svgbob** (Rust).  A tool that converts ASCII diagram scribbles into SVG images.  It is often integrated into documentation generators and can be used with Markdown.  While it works in the opposite direction (ASCII to vector), its parser demonstrates how to recognise lines, arrows and shapes from characters and may provide ideas for designing an ASCII drawing library.

- **rasciichart / asciigraph** (Rust).  Libraries that draw line charts in the terminal using ASCII or Unicode characters.  They provide smooth line rendering and configurable axes【430229306468881†L94-L104】.  These libraries are useful for plotting numeric data (e.g., XY charts) but not for general diagram rendering.

## 5 Recommendations for building a Rust ASCII renderer

1. **Define scope**: start with **flowcharts** and **sequence diagrams**, as they are widely used and their layout rules are manageable.  Support for state diagrams and class diagrams can be added later.

2. **Parse Mermaid**: choose between embedding the JavaScript parser or writing a subset parser in Rust.  For quick results, call the Mermaid CLI (`mmdc`) to convert Mermaid to SVG/JSON and extract the layout information.

3. **Implement or reuse a layout engine**: for an initial version, call Graphviz’s `dot` or use the `elk` command‑line tool to compute node positions.  Later, implement a simplified Sugiyama algorithm as described above.

4. **Design the rendering layer**: implement a `Canvas` type that maintains a 2‑D array of `char`/`String` and exposes functions to draw boxes, lines and labels.  Provide configuration options: ASCII vs Unicode, padding, arrow style.  Add a second pass to adjust spacing.

5. **Support multiple diagram types** via an enum and trait implementations for parsing and layout.  Use `enum Diagram` with variants `Flowchart`, `Sequence`, etc., each with its own `layout()` method.

6. **Leverage existing libraries**: study **mermaid‑ascii** for handling edge cases like labelled edges or nested subgraphs and **beautiful‑mermaid** for themeable output.  Use `unicode‑width` crate to compute cell widths and `petgraph` for graph operations.

7. **Iterate**: after basic support is working, add features like adjustable themes, extended shapes (diamonds, hexagons), dashed edges, and support for international text.  Provide options to export diagrams to other formats (SVG via svgbob) if needed.

## 6 Conclusion

Rendering Mermaid diagrams to ASCII is primarily a graph layout and rendering problem.  You must parse the diagram into an AST, compute node sizes, apply suitable layout algorithms (e.g., Sugiyama for flowcharts, simple row/column layouts for sequences), and draw the result on a monospaced character grid.  ASCII’s limitations necessitate orthogonal routing, simple shapes and fixed‑width fonts, but with careful spacing and Unicode box‑drawing characters you can produce clear diagrams.  Studying existing tools like **mermaid‑ascii** (which demonstrates full support for flowchart and sequence diagrams and provides options for spacing and ASCII vs Unicode output【665572273740704†L360-L407】【665572273740704†L560-L637】) and **beautiful‑mermaid** (which exposes `renderMermaidAscii()` with configurable options【312636815307403†L156-L174】) will provide practical insights.  For a Rust implementation, combining graph libraries (`petgraph`), parsing crates (`nom` or `pest`), and external layout engines (Graphviz or ELK) is a pragmatic approach.  Begin with a limited subset and incrementally extend support for more diagram types and features.

