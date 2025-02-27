# Motoko compiler changelog

* Add type union and intersection. The type expression

  ```motoko
  T and U
  ```
  produces the greatest lower bound of types `T` and `U`, that is,
  the greatest type that is a subtype of both. Dually,

  ```motoko
  T or U
  ```
  produces the least upper bound of types `T` and `U`, that is,
  the smallest type that is a supertype of both.

  One use case of the former is "extending" an existing object type:

  ``` motoko
  type Person = {name : Text; address : Text};
  type Manager = Person and {underlings : [Person]};
  ```
  Similarly, the latter can be used to "extend" a variant type:
  ```motoko
  type Workday = {#mon; #tue; #wed; #thu; #fri};
  type Weekday = Workday or {#sat; #sun};
  ```

== 0.6.11 (2021-10-08)

* Assertion error messages are now reproducible (#2821)

== 0.6.10 (2021-09-23)

* moc

  * documentation changes

* motoko-base

  * documentation changes

== 0.6.9 (2021-09-15)

* motoko-base

  * add Debug.trap : Text -> None (#288)

== 0.6.8 (2021-09-06)

* Introduce primitives for `Int` ⇔ `Float` conversions (#2733)
* Bump LLVM toolchain to version 12 (#2542)
* Support extended name linker sections (#2760)
* Fix crashing bug for formatting huge floats (#2737)

== 0.6.7 (2021-08-16)

* moc

  *  Optimize field access by exploiting field ordering (#2708)
  *  Fix handling of self references in mark-compact GC (#2721)
  *  Restore CI reporting of perf-regressions (#2643)

* motoko-base:

  * Fix bug in `AssocList.diff` (#277)
  * Deprecate unsafe or redundant functions in library `Option` ( `unwrap`, `assertSome`, `assertNull`) (#275)

== 0.6.6 (2021-07-30)

* Vastly improved garbage collection scheduling: previously Motoko runtime would do GC
  after every update message. We now schedule a GC when

  1. Heap grows more than 50% and 10 MiB since the last GC, or
  2. Heap size is more than 3 GiB

  (1) is to make sure we don't do GC on tiny heaps or after only small amounts of allocation.
  (2) is to make sure that on large heaps we will have enough allocation space during the next message.

  This scheduling reduces cycles substantially, but may moderately increase memory usage.

  New flag `--force-gc` restores the old behavior.

* Fix bug in compacting gc causing unnecessary memory growth (#2673)

* Trap on attempt to upgrade when canister not stopped and there are outstanding callbacks.
  (This failure mode can be avoided by stopping the canister before upgrade.)

* Fix issue #2640 (leaked `ClosureTable` entry when awaiting futures fails).

== 0.6.5 (2021-07-08)

* Add alternative, _compacting_ gc, enabled with new moc flag `--compacting-gc`.
  The compacting gc supports larger heap sizes than the default, 2-space copying collector.

  NOTE: Dfx 0.7.6 adds optional field `"args"` to `dfx.json` files,
  so Motoko canisters can specify `moc` command-line arguments. E.g.,

  ```json
  ...
     "type" : "motoko"
     ...
     "args" : "--compacting-gc"
  ...
  ```

* Documentation fixes.
* Command line tools: `--help` option provides better documentation of command line
  options that have arguments.
* Fix issue #2319 (crash on import of Candid class).

== 0.6.4 (2021-06-12)

* For release builds, the banner (`moc --version`) now includes the release
  version.

* Fix MacOS release builds (the 0.6.3 tarball for MacOS contained the linux binaries)

== 0.6.3 (2021-06-10)

* Motoko is now open source!

* Better internal consistency checking of the intermediate representation

== 0.6.2 (2021-05-24)

* motoko-base:

  * reformat to style guidelines
  * add type bindings `Nat.Nat`, `Nat8.Nat8` etc. to libraries for primitive types.

* Bugfix: generation of candid from Motoko:

  * no longer confused by distinct, but eponymous, type definitions (Bug: #2529);
  * numbers eponymous types and specializations from 1 (not 2);
  * avoids long chains of type equalities by normalizing before translation.

== 0.6.1 (2021-04-30)

* Internal: Update to IC interface spec 0.17 (adapt to breaking change to signature of `create_canister`)

== 0.6.0 (2021-04-16)

* BREAKING CHANGE:
  The old-style object and block syntax deprecated in 0.5.0 is finally removed.

* Record punning: As already supported in patterns, short object syntax in
  expressions now allows omitting the right-hand side if it is an identifier
  of the same name as the label. That is,

  ```motoko
  {a; b = 1; var c}
  ```

  is short for

  ```motoko
  {a = a; b = 1; var c = c}
  ```

  assuming respective variables are in scope.

* BREAKING CHANGE:
  The types `Word8`, `Word16`, `Word32` and `Word64` have been removed.
  This also removed the `blob.bytes()` iterator.

  Motoko base also dropped the `Word8`, `Word16`, `Word32` and `Word64`
  modules.

  This concludes the transition to the other fixed-width types that began with
  version 0.5.8

* BREAKING CHANGE (Minor):
 `await` on a completed future now also commits state and suspends
  computation, to ensure every await, regardless of its future's state,
  is a commit point for state changes and tentative message sends.

  (Previously, only awaits on pending futures would force a commit
   and suspend, while awaits on completed futures would continue
   execution without an incremental commit, trading safety for speed.)

* motoko-base: fixed bug in `Text.compareWith`.

== 0.5.15 (2021-04-13)

* Bugfix: `Blob.toArray` was broken.

== 0.5.14 (2021-04-09)

* BREAKING CHANGE (Minor): Type parameter inference will no longer default
  under-constrained type parameters that are invariant in the result, but
  require an explicit type argument.
  This is to avoid confusing the user by inferring non-principal types.

  For example, given (invariant) class `Box<A>`:

  ```motoko
    class Box<A>(a : A) { public var value = a; };
  ```

  the code

  ```motoko
    let box = Box(0); // rejected
  ```

  is rejected as ambiguous and requires an instantiation, type annotation or
  expected type. For example:

  ```motoko
    let box1 = Box<Int>(0); // accepted
    let box2 : Box<Nat> = Box(0); // accepted
  ```

  Note that types `Box<Int>` and `Box<Nat>` are unrelated by subtyping,
  so neither is best (or principal) in the ambiguous, rejected case.

* Bugfix: Type components in objects/actors/modules correctly ignored
  when involved in serialization, equality and `debug_show`, preventing
  the compiler from crashing.

* motoko-base: The `Text.hash` function was changed to a better one.
  If you stored hashes as stable values (which you really shouldn't!)
  you must rehash after upgrading.

* motoko-base: Conversion functions between `Blob` and `[Nat8]` are provided.

* When the compiler itself crashes, it will now ask the user to report the
  backtrace at the DFINITY forum

== 0.5.13 (2021-03-25)

* The `moc` interpreter now pretty-prints values (as well as types) in the
  repl, producing more readable output for larger values.

* The family of `Word` types are deprecated, and mentioning them produces a warning.
  These type will be removed completely in a subsequent release.
  See the user’s guide, section “Word types”, for a migration guide.

* motoko base: because of this deprecation, the `Char.from/toWord32()`
  functions are removed. Migrate away from `Word` types, or use
  `Word32.from/ToChar` for now.

== 0.5.12 (2021-03-23)

* The `moc` compiler now pretty-prints types in error messages and the repl,
  producing more readable output for larger types.

* motoko base: fixed bug in `Text.mo` affecting partial matches in,
  for example, `Text.replace` (GH issue #234).

== 0.5.11 (2021-03-12)

* The `moc` compiler no longer rejects occurrences of private or
  local type definitions in public interfaces.

  For example,

  ```motoko
  module {
    type List = ?(Nat, List); // private
    public func cons(n : Nat, l : List) : List { ?(n , l) };
  }
  ```

  is now accepted, despite `List` being private and appearing in the type
  of public member `cons`.

* Type propagation for binary operators has been improved. If the type of one of
  the operands can be determined locally, then the other operand is checked
  against that expected type. This should help avoiding tedious type annotations
  in many cases of literals, e.g., `x == 0` or `2 * x`, when `x` has a special
  type like `Nat8`.

* The `moc` compiler now rejects type definitions that are non-_productive_ (to ensure termination).

  For example, problematic types such as:

  ```motoko
  type C = C;
  type D<T, U> = D<U, T>;
  type E<T> = F<T>;
  type F<T> = E<T>;
  type G<T> = Fst<G<T>, Any>;
  ```

  are now rejected.

* motoko base: `Text` now contains `decodeUtf8` and `encodeUtf8`.

== 0.5.10 (2021-03-02)

* User defined deprecations

  Declarations in modules can now be annotated with a deprecation comment, which make the compiler emit warnings on usage.

  This lets library authors warn about future breaking changes:

  As an example:

  ```motoko
  module {
    /// @deprecated Use `bar` instead
    public func foo() {}

    public func bar() {}
  }
  ```

  will emit a warning whenever `foo` is used.

* The `moc` compiler now rejects type definitions that are _expansive_, to help ensure termination.
  For example, problematic types such as `type Seq<T> = ?(T, Seq<[T]>)` are rejected.

* motoko base: `Time.Time` is now public

== 0.5.9 (2021-02-19)

* The `moc` compiler now accepts the `-Werror` flag to turn warnings into errors.

* The language server now returns documentation comments alongside
  completions and hover notifications

== 0.5.8 (2021-02-12)

* Wrapping arithmetic and bit-wise operations on `NatN` and `IntN`

  The conventional arithmetic operators on `NatN` and `IntN` trap on overflow.
  If wrap-around semantics is desired, the operators `+%`, `-%`, `*%` and `**%`
  can be used. The corresponding assignment operators (`+%=` etc.) are also available.

  Likewise, the bit fiddling operators (`&`, `|`, `^`, `<<`, `>>`, `<<>`,
  `<>>` etc.) are now also available on `NatN` and `IntN`. The right shift
  operator (`>>`) is an unsigned right shift on `NatN` and a signed right shift
  on `IntN`; the `+>>` operator is _not_ available on these types.

  The motivation for this change is to eventually deprecate and remove the
  `WordN` types.

  Therefore, the wrapping arithmetic operations on `WordN` are deprecated and
  their use will print a warning. See the user’s guide, section “Word types”,
  for a migration guide.

* For values `x` of type `Blob`, an iterator over the elements of the blob
  `x.vals()` is introduced. It works like `x.bytes()`, but returns the elements
  as type `Nat8`.

* `mo-doc` now generates cross-references for types in signatures in
  both the Html as well as the Asciidoc output. So a signature like
  `fromIter : I.Iter<Nat> -> List.List<Nat>` will now let you click on
  `I.Iter` or `List.List` and take you to their definitions.

* Bugfix: Certain ill-typed object literals are now prevented by the type
  checker.

* Bugfix: Avoid compiler aborting when object literals have more fields than
  their type expects.

== 0.5.7 (2021-02-05)

* The type checker now exploits the expected type, if any,
  when typing object literal expressions.
  So `{ x = 0 } : { x : Nat8 }` now works as expected
  instead of requiring an additional type annotation on `0`.

== 0.5.6 (2021-01-22)

* The compiler now reports errors and warnings with an additional _error code_
  This code can be used to look up a more detailed description for a given error by passing the `--explain` flag with a code to the compiler.
  As of now this isn't going to work for most codes because the detailed descriptions still have to be written.
* Internal: The parts of the RTS that were written in C have been ported to Rust.

== 0.5.5 (2021-01-15)

* new `moc` command-line arguments `--args <file>` and `--args0 <file>` for
  reading newline/NUL terminated arguments from `<file>`.
* motoko base: documentation examples are executable in the browser

== 0.5.4 (2021-01-07)

* _Option blocks_ `do ? <block>` and _option checks_ `<exp> !`.
  Inside an option block, an option check validates that its operand expression is not `null`.
  If it is, the entire option block is aborted and evaluates to `null`.
  This simplifies consecutive null handling by avoiding verbose `switch` expressions.

  For example, the expression `do? { f(x!, y!) + z!.a }` evaluates to `null` if either `x`, `y` or `z` is `null`;
  otherwise, it takes the options' contents and ultimately returns `?r`, where `r` is the result of the addition.

* BREAKING CHANGE (Minor):
  The light-weight `do <exp>` form of the recently added, more general `do <block-or-exp>` form,
  is no longer legal syntax.
  That is, the argument to a `do` or `do ?` expression *must* be a block `{ ... }`,
  never a simple expression.

== 0.5.3 (2020-12-10)

* Nothing new, just release moc.js to CDN

== 0.5.2 (2020-12-04)

* Bugfix: gracefully handle importing ill-typed actor classes

== 0.5.1 (2020-11-27)

* BREAKING CHANGE: Simple object literals of the form `{a = foo(); b = bar()}`
  no longer bind the field names locally. This enables writing expressions
  like `func foo(a : Nat) { return {x = x} }`.

  However, this breaks expressions like `{a = 1; b = a + 1}`. Such object
  shorthands now have to be written differently, e.g., with an auxiliary
  declaration, as in `let a = 1; {a = a; b = a + 1}`, or by using the "long"
  object syntax `object {public let a = 1; public let b = a + 1}`.

== 0.5.0 (2020-11-27)

* BREAKING CHANGE: Free-standing blocks are disallowed

  Blocks are only allowed as sub-expressions of control flow expressions like
  `if`, `loop`, `case`, etc. In all other places, braces are always considered
  to start an object literal.

  To use blocks in other positions, the new `do <block>` expression can be
  used.

  The more liberal syntax is still allowed for now but deprecated, i.e.,
  produces a warning.

* BREAKING CHANGE: actor creation is regarded as asynchronous:

  * Actor declarations are asynchronous and can only be used in asynchronous
    contexts.
  * The return type of an actor class, if specified, must be an async actor
    type.
  * To support actor declaration, the top-level context of an interpreted
    program is an asynchronous context, allowing implicit and explicit await
    expressions.

  (Though breaking, this change mostly affects interpreted programs and
  compiled programs with explicate actor class return types)

* Candid support is updated to latest changes of the Candid spec, in particular
  the ability to extend function with optional parameters in a backward
  compatible way.

  Motoko passes the official Candid compliance test suite.

* RTS: Injecting a value into an option type (`? <exp>`) no longer
  requires heap allocation in most cases. This removes the memory-tax
  of using iterators.

* Bugfix: Passing cycles to the instantiation of an actor class works now.

* Various bug fixes and documentation improvements.

== 0.4.6 (2020-11-13)

* Significant documentation improvements
* Various bugfixes
* Improved error messages
* Initial DWARF support
* Candid compliance improvements:
  * Strict checking of utf8 strings
  * More liberal parsing of leb128-encoded numbers
* New motoko-base:
  * The Random library is added

== 0.4.5 (2020-10-06)

* BREAKING CHANGE: a library containing a single actor class is
  imported as a module, providing access to both the class type and
  class constructor function as module components. Restores the
  invariant that imported libraries are modules.
* Backend: Compile captured actor class parameters statically (#2022)
* flip the default for -g (#1546)
* Bug fix: reject array indexing as non-static (could trap) (#2011)
* Initialize tuple length fields (#1992)
* Warns for structural equality on abstract types (#1972)
* Funds Imperative API (#1922)
* Restrict subtyping (#1970)
* Continue labels always have unit codomain (#1975)
* Compile.ml: target and use new builder call pattern (#1974)
* fix scope var bugs (#1973)

== 0.4.4 (2020-09-21)

* Actor class export
* Accept unit installation args for actors
* Reject platform actor (class) programs with additional decs
* Handle IO exceptions at the top-level
* RTS: Remove duplicate array and blob allocation code
* RTS: Fix pointer arithmetic in BigInt collection function

== 0.4.3 (2020-09-14)

* Preliminary support for actor class import and dynamic canister installation.
  Surface syntax may change in future.
* BREAKING CHANGE: a compilation unit/file defining an actor or actor class may *only* have leading `import` declarations; other leading declarations (e.g. `let` or `type`) are no longer supported.
* Rust GC

== 0.4.2 (2020-08-18)

* Polymorphic equality.  `==` and `!=` now work on all shareable types.

== 0.4.1 (2020-08-13)

* Switching to bumping the third component of the version number
* Bugfix: clashing declarations via function and class caught (#1756)
* Bugfix: Candid `bool` decoding rejects invalid input (#1783)
* Canisters can take installation arguments (#1809)
  NB: Communicating the type of the canister installation methods is still
  missing.
* Optimization: Handling of `Bool` in the backend.

== 0.4 (2020-08-03)

* Candid pretty printer to use shorthand when possible (#1774)
* fix candid import to use the new id format (#1787)

== 0.3 (2020-07-31)

* Fixes an issue with boolean encoding to Candid
* Converts the style guide to asciidocs

== 0.2 (2020-07-30)

* The `Blob` type round-trips through candid type export/import (#1744)
* Allow actor classes to know the caller of their constructor (#1737)
* Internals: `Prim.time()` provided (#1747)
* Performance: More dead code removal (#1752)
* Performance: More efficient arithmetic with unboxed values (#1693, #1757)
* Canister references are now parsed and printed according to the new
  base32-based textual format (#1732).
* The runtime is now embedded into `moc` and need not be distributed separately
  (#1772)

== 0.1 (2020-07-20)

* Beginning of the changelog. Released with dfx-0.6.0.
